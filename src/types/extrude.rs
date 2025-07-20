use super::*;

impl TypeEnv {
    pub(crate) fn extrude(
        &mut self,
        ty: InferedTypeId,
        polarity: Polarity,
        level: usize,
    ) -> InferedTypeId {
        self.extrude_inner(ty, polarity, level, &mut HashMap::new())
    }

    fn extrude_inner(
        &mut self,
        ty: InferedTypeId,
        polarity: Polarity,
        level: usize,
        cache: &mut HashMap<PolarVariable, InferedTypeId>,
    ) -> InferedTypeId {
        let ty_level = ty.level(self);
        tracing::trace!(
            "Extruding ID:{} ({:?}) - level {} into level {}",
            ty.0,
            polarity,
            ty_level,
            level
        );
        if ty_level <= level {
            tracing::trace!("Type level {ty_level} <= level {level}");
            return ty;
        }

        let infered = self.get(ty);
        tracing::trace!("Extruding Type: {:?}", infered);
        match infered {
            InferedType::Error { .. }
            | InferedType::Primitive { .. }
            | InferedType::Literal { .. } => ty,
            &InferedType::Variable { id, span } => {
                let tv_pol = PolarVariable { polarity, id };
                match cache.get(&tv_pol).copied() {
                    Some(id) => {
                        tracing::trace!("Variable exists in cache. Reusing: {}", id.0);
                        id
                    }
                    None => {
                        let nvs = self.fresh_var(span, level);
                        tracing::trace!("Extruding new variable: {}", nvs.0);
                        cache.insert(tv_pol, nvs);
                        match polarity {
                            Polarity::Positive => {
                                tracing::trace!(
                                    "Inserting NV {} into upper bounds of VAR{}",
                                    nvs.0,
                                    id.0
                                );
                                self.vars[id.0].upper_bounds.push(nvs);
                                tracing::trace!(
                                    "Extruding lower bounds of VAR{}: {:?}",
                                    id.0,
                                    self.vars[id.0].lower_bounds
                                );
                                let lower_bounds = self.vars[id.0]
                                    .lower_bounds
                                    .clone()
                                    .into_iter()
                                    .map(|lb| self.extrude_inner(lb, polarity, level, cache))
                                    .collect();
                                tracing::trace!(
                                    "Updating lower bounds of NV {} with extruded {lower_bounds:?}",
                                    nvs.0
                                );
                                self.vars_of(nvs).lower_bounds = lower_bounds;
                            }
                            Polarity::Negative => {
                                tracing::trace!(
                                    "Inserting NV {} into lower bounds of VAR{}",
                                    nvs.0,
                                    id.0
                                );
                                self.vars[id.0].lower_bounds.push(nvs);

                                tracing::trace!(
                                    "Extruding upper bounds of VAR{}: {:?}",
                                    id.0,
                                    self.vars[id.0].upper_bounds
                                );
                                let upper_bounds = self.vars[id.0]
                                    .upper_bounds
                                    .clone()
                                    .into_iter()
                                    .map(|ub| self.extrude_inner(ub, polarity, level, cache))
                                    .collect();
                                tracing::trace!(
                                    "Updating upper bounds of NV {} with extruded {upper_bounds:?}",
                                    nvs.0
                                );
                                self.vars_of(nvs).upper_bounds = upper_bounds;
                            }
                        }
                        nvs
                    }
                }
            }
            &InferedType::Function { lhs, rhs, span } => {
                let lhs = self.extrude_inner(lhs, polarity.negate(), level, cache);
                let rhs = self.extrude_inner(rhs, polarity, level, cache);
                self.function(lhs, rhs, span)
            }
            &InferedType::Applicative {
                arg,
                ret,
                first_arg,
                span,
            } => {
                let arg = self.extrude_inner(arg, polarity.negate(), level, cache);
                let ret = self.extrude_inner(ret, polarity, level, cache);
                let first_arg = first_arg.map(|first_arg| {
                    self.extrude_inner(first_arg, polarity.negate(), level, cache)
                });
                self.applicative(arg, ret, first_arg, span)
            }
            &InferedType::Tuple {
                ref items,
                rest,
                span,
            } => {
                let items = items
                    .clone()
                    .into_iter()
                    .map(|item| self.extrude_inner(item, polarity, level, cache))
                    .collect();
                let rest = rest.map(|rest| self.extrude_inner(rest, polarity, level, cache));
                self.tuple(items, rest, span)
            }
            &InferedType::Record {
                ref fields,
                proto,
                span,
            } => {
                let fields = fields
                    .clone()
                    .into_iter()
                    .map(|(name, ty)| {
                        let ty = self.extrude_inner(ty, polarity, level, cache);
                        (name, ty)
                    })
                    .collect();
                let proto = proto.map(|proto| self.extrude_inner(proto, polarity, level, cache));
                self.record(fields, proto, span)
            }
            &InferedType::List { item, span } => {
                let item = self.extrude_inner(item, polarity, level, cache);
                self.list(item, span)
            }
            &InferedType::Ref { write, read, span } => {
                let write = write.map(|write| self.extrude_inner(write, polarity, level, cache));
                let read = read.map(|read| self.extrude_inner(read, polarity, level, cache));
                self.reference(read, write, span)
            }
            &InferedType::Module { ref members, span } => {
                let members = members
                    .clone()
                    .into_iter()
                    // .map(|(name, scheme)| {
                    //     // TODO: What to do with module extrusion
                    //     // let ty = scheme.instantiate(self, level);
                    //     // let ty = self.extrude_inner(ty, polarity, level, cache);
                    //     (name, scheme)
                    // })
                    .collect();
                self.module(members, span)
            }
        }
    }
}
