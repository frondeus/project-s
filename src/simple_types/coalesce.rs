use super::{utils::variable_letters, *};

#[derive(Default)]
struct VarGenerator {
    counter: usize,
    map: HashMap<VarId, usize>,
}

impl VarGenerator {
    fn new_var(&mut self, var: VarId) -> String {
        let id = self.map.entry(var).or_insert_with(|| {
            let c = self.counter;
            self.counter += 1;
            c
        });

        let name = variable_letters(*id);
        tracing::trace!("Creating new type var: {name}");
        name
    }
}

impl TypeEnv {
    pub fn coalesce(&mut self, ty: InferedTypeId) -> TypeId {
        let mut recursive: HashMap<PolarVariable, String> = HashMap::new();

        self.coalesce_inner(
            ty,
            Polarity::Positive,
            &mut recursive,
            &mut HashSet::new(),
            &mut VarGenerator::default(),
        )
    }

    fn coalesce_scheme(
        &mut self,
        scheme: InferedTypeScheme,
        polarity: Polarity,
        recursive: &mut HashMap<PolarVariable, String>,
        in_process: &mut HashSet<PolarVariable>,
        vars: &mut VarGenerator,
    ) -> TypeScheme {
        match scheme {
            InferedTypeScheme::Monomorphic(infered_type_id) => {
                let ty =
                    self.coalesce_inner(infered_type_id, polarity, recursive, in_process, vars);
                TypeScheme::Monomorphic(ty)
            }
            InferedTypeScheme::Polymorphic(infered_polymorphic_type) => {
                let body = self.coalesce_inner(
                    infered_polymorphic_type.body,
                    polarity,
                    recursive,
                    in_process,
                    vars,
                );
                TypeScheme::Polymorphic(PolymorphicType {
                    level: infered_polymorphic_type.level,
                    body,
                })
            }
        }
    }

    fn coalesce_bounds(
        &mut self,
        id: VarId,
        tv_pol: PolarVariable,
        polarity: Polarity,
        recursive: &mut HashMap<PolarVariable, String>,
        in_process: &mut HashSet<PolarVariable>,
        vars: &mut VarGenerator,
    ) -> Vec<TypeId> {
        let vs_vars = &self.vars[id.0];
        tracing::trace!("VS_VARS: {:#?}", vs_vars);
        let bounds = match polarity {
            Polarity::Positive => {
                tracing::trace!("Polarity positive. Using lower bounds");
                vs_vars.lower_bounds.clone()
            }
            Polarity::Negative => {
                tracing::trace!("Polarity negative. Using upper bounds");
                vs_vars.upper_bounds.clone()
            }
        };
        tracing::trace!("Bounds: {:#?}", bounds);
        let mut bound_types = bounds
            .into_iter()
            .map(|bound| {
                in_process.insert(tv_pol);
                let ty = self.coalesce_inner(bound, polarity, recursive, in_process, vars);
                in_process.remove(&tv_pol);
                ty
            })
            .collect::<Vec<_>>();
        tracing::trace!("Bound types: {:#?}", bound_types);

        bound_types.sort();
        bound_types.dedup();

        bound_types
    }

    fn coalesce_inner(
        &mut self,
        ty_id: InferedTypeId,
        polarity: Polarity,
        recursive: &mut HashMap<PolarVariable, String>,
        in_process: &mut HashSet<PolarVariable>,
        vars: &mut VarGenerator,
    ) -> TypeId {
        let ty = &self.infered[ty_id.0];
        tracing::trace!("Coalesce({} ; {:?}): {:?}", ty_id.0, polarity, ty);
        match ty {
            InferedType::Error { span: _ } => self.add_type(Type::Error),
            &InferedType::Variable { id, span: _ } => {
                let tv_pol = PolarVariable { id, polarity };
                if in_process.contains(&tv_pol) {
                    let name = recursive
                        .entry(tv_pol)
                        .or_insert_with(|| vars.new_var(id))
                        .clone();
                    self.add_type(Type::Variable { name })
                } else {
                    let mut bound_types =
                        self.coalesce_bounds(id, tv_pol, polarity, recursive, in_process, vars);

                    // if bound_types.is_empty() && polarity == Polarity::Negative {
                    //     bound_types.extend(self.coalesce_bounds(
                    //         id,
                    //         tv_pol,
                    //         Polarity::Positive,
                    //         recursive,
                    //         in_process,
                    //         vars,
                    //     ));
                    // }

                    if bound_types.is_empty() {
                        tracing::trace!("Bounds are empty. {:?}", tv_pol);
                        let name = recursive
                            .entry(tv_pol)
                            .or_insert_with(|| vars.new_var(id))
                            .clone();
                        return self.add_type(Type::Variable { name });
                    }
                    if bound_types.len() == 1 {
                        let ty = bound_types.pop().unwrap();

                        return match recursive.get(&tv_pol) {
                            Some(name) => self.add_type(Type::Recursive {
                                name: name.clone(),
                                body: ty,
                            }),
                            None => ty,
                        };
                    }

                    let res = match polarity {
                        Polarity::Positive => self.add_type(Type::Union { items: bound_types }),
                        Polarity::Negative => {
                            self.add_type(Type::Intersection { items: bound_types })
                        }
                    };

                    match recursive.get(&tv_pol) {
                        Some(name) => self.add_type(Type::Recursive {
                            name: name.clone(),
                            body: res,
                        }),
                        None => res,
                    }
                }
            }
            InferedType::Primitive { name, span: _ } => {
                self.add_type(Type::Primitive { name: name.clone() })
            }
            InferedType::Literal { value, span: _ } => self.add_type(Type::Literal {
                value: value.clone(),
            }),
            &InferedType::Function { lhs, rhs, span: _ } => {
                let lhs = self.coalesce_inner(lhs, polarity.negate(), recursive, in_process, vars);
                let rhs = self.coalesce_inner(rhs, polarity, recursive, in_process, vars);
                self.add_type(Type::Function { lhs, rhs })
            }
            &InferedType::Applicative {
                arg,
                ret,
                first_arg,
                span: _,
            } => {
                let arg = self.coalesce_inner(arg, polarity.negate(), recursive, in_process, vars);
                let ret = self.coalesce_inner(ret, polarity, recursive, in_process, vars);
                let first_arg = first_arg.map(|arg| {
                    self.coalesce_inner(arg, polarity.negate(), recursive, in_process, vars)
                });
                self.add_type(Type::Applicative {
                    arg,
                    ret,
                    first_arg,
                })
            }
            InferedType::Tuple { items, span: _ } => {
                let items = items
                    .clone()
                    .into_iter()
                    .map(|ty| self.coalesce_inner(ty, polarity, recursive, in_process, vars))
                    .collect();
                self.add_type(Type::Tuple { items })
            }
            InferedType::Record {
                fields,
                proto,
                span: _,
            } => {
                let proto = *proto;
                let mut fields: IndexMap<String, TypeId> = fields
                    .clone()
                    .into_iter()
                    .map(|(name, ty)| {
                        let ty = self.coalesce_inner(ty, polarity, recursive, in_process, vars);
                        (name, ty)
                    })
                    .collect();

                if let Some(proto) = proto {
                    let proto = self.coalesce_inner(proto, polarity, recursive, in_process, vars);
                    let p_fields = match &self.types[proto.0] {
                        Type::Record {
                            fields: p_fields,
                            proto: _,
                        } => p_fields,
                        _ => {
                            return self.add_type(Type::Record {
                                fields,
                                proto: Some(proto),
                            });
                        }
                    };
                    for (field_name, field_ty) in p_fields {
                        if fields.iter().all(|(n, _)| n != field_name) {
                            fields.insert(field_name.clone(), *field_ty);
                        }
                    }
                }

                self.add_type(Type::Record {
                    fields,
                    proto: None,
                })
            }
            &InferedType::List { item, span: _ } => {
                let item = self.coalesce_inner(item, polarity, recursive, in_process, vars);
                self.add_type(Type::List { item })
            }
            &InferedType::Ref {
                write,
                read,
                span: _,
            } => {
                let write = write
                    .map(|write| self.coalesce_inner(write, polarity, recursive, in_process, vars));
                let read = read.map(|read| {
                    self.coalesce_inner(read, polarity.negate(), recursive, in_process, vars)
                });
                self.add_type(Type::Ref { write, read })
            }
            InferedType::Module { members, span: _ } => {
                let members = members
                    .clone()
                    .into_iter()
                    .map(|(name, scheme)| {
                        let ty =
                            self.coalesce_scheme(scheme, polarity, recursive, in_process, vars);
                        (name, ty)
                    })
                    .collect();

                self.add_type(Type::Module { members })
            }
        }
    }
}
