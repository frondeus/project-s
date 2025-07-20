use super::*;

impl InferedTypeScheme {
    pub(crate) fn instantiate(&self, type_env: &mut TypeEnv, level: usize) -> InferedTypeId {
        match self {
            InferedTypeScheme::Monomorphic(id) => *id,
            InferedTypeScheme::Polymorphic(poly) => {
                InferedPolymorphicType::freshen_above(type_env, poly.level, poly.body, level)
            }
        }
    }
}

impl InferedPolymorphicType {
    pub fn freshen_above(
        type_env: &mut TypeEnv,
        limit: usize,
        ty: InferedTypeId,
        level: usize,
    ) -> InferedTypeId {
        let mut freshened = HashMap::new();

        tracing::trace!(
            "Freshen above: Limit: {limit} ; TY: N{} ; LVL {level}",
            ty.0
        );
        let res = Self::freshen(type_env, ty, limit, level, &mut freshened);
        tracing::trace!("Freshened: {} -> {}", ty.0, res.0);

        res
    }

    fn freshen(
        type_env: &mut TypeEnv,
        ty: InferedTypeId,
        limit: usize,
        level: usize,
        freshened: &mut HashMap<VarId, InferedTypeId>,
    ) -> InferedTypeId {
        let ty_level = ty.level(type_env);
        tracing::trace!(
            "Freshening N{} - lvl {ty_level} ; limit {limit} ; level {level}",
            ty.0,
        );
        if ty_level <= limit {
            tracing::trace!(
                "Type level {} lower or equal than limit {}",
                ty_level,
                limit
            );
            ty
        } else {
            let infered = type_env.get(ty);
            tracing::trace!("Freshening {:?}", infered);
            match infered {
                InferedType::Error { .. }
                | InferedType::Primitive { .. }
                | InferedType::Literal { .. } => ty,
                &InferedType::Variable { id, span } => match freshened.get(&id) {
                    Some(v) => *v,
                    None => {
                        let v = type_env.fresh_var(span, level);
                        tracing::trace!("Creating new fresh variable: {}", v.0);
                        freshened.insert(id, v);

                        let tvs = type_env.vars[id.0].clone();
                        let lower_bounds = tvs
                            .lower_bounds
                            .into_iter()
                            .rev()
                            .map(|lb| Self::freshen(type_env, lb, limit, level, freshened))
                            .rev()
                            .collect();

                        let upper_bounds = tvs
                            .upper_bounds
                            .into_iter()
                            .rev()
                            .map(|ub| Self::freshen(type_env, ub, limit, level, freshened))
                            .rev()
                            .collect();

                        let vars = type_env.vars_of(v);
                        vars.lower_bounds = lower_bounds;
                        vars.upper_bounds = upper_bounds;

                        v
                    }
                },
                &InferedType::Function { lhs, rhs, span } => {
                    let lhs = Self::freshen(type_env, lhs, limit, level, freshened);
                    let rhs = Self::freshen(type_env, rhs, limit, level, freshened);
                    type_env.function(lhs, rhs, span)
                }
                &InferedType::Applicative {
                    arg,
                    ret,
                    first_arg,
                    span,
                } => {
                    let arg = Self::freshen(type_env, arg, limit, level, freshened);
                    let ret = Self::freshen(type_env, ret, limit, level, freshened);
                    let first_arg = first_arg.map(|first_arg| {
                        Self::freshen(type_env, first_arg, limit, level, freshened)
                    });
                    type_env.applicative(arg, ret, first_arg, span)
                }
                &InferedType::Tuple {
                    ref items,
                    rest,
                    span,
                } => {
                    let items = items
                        .clone()
                        .into_iter()
                        .map(|item| Self::freshen(type_env, item, limit, level, freshened))
                        .collect();
                    let rest =
                        rest.map(|rest| Self::freshen(type_env, rest, limit, level, freshened));
                    type_env.tuple(items, rest, span)
                }
                InferedType::Record {
                    fields,
                    proto,
                    span,
                } => {
                    let span = *span;
                    let fields = fields.clone();
                    let proto = (*proto)
                        .map(|proto| Self::freshen(type_env, proto, limit, level, freshened));
                    let fields = fields
                        .into_iter()
                        .map(|(name, ty)| {
                            let ty = Self::freshen(type_env, ty, limit, level, freshened);
                            (name, ty)
                        })
                        .collect();
                    type_env.record(fields, proto, span)
                }
                &InferedType::List { item, span } => {
                    let item = Self::freshen(type_env, item, limit, level, freshened);
                    type_env.list(item, span)
                }
                &InferedType::Ref { write, read, span } => {
                    let write =
                        write.map(|write| Self::freshen(type_env, write, limit, level, freshened));
                    let read =
                        read.map(|read| Self::freshen(type_env, read, limit, level, freshened));
                    type_env.reference(read, write, span)
                }
                &InferedType::Module { .. } => ty,
            }
        }
    }
}
