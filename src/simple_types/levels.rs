use super::*;

pub trait WithLevel {
    fn level(&self, type_env: &TypeEnv) -> usize;
}
impl WithLevel for InferedTypeId {
    fn level(&self, type_env: &TypeEnv) -> usize {
        match type_env.get(*self) {
            InferedType::Error { .. }
            | InferedType::Primitive { .. }
            | InferedType::Literal { .. } => 0,
            InferedType::Variable { id, span: _ } => {
                let var = &type_env.vars[id.0];
                var.level
            }
            InferedType::Function { lhs, rhs, span: _ } => {
                lhs.level(type_env).max(rhs.level(type_env))
            }
            InferedType::Applicative {
                arg,
                ret,
                first_arg: _,
                span: _,
            } => arg.level(type_env).max(ret.level(type_env)),
            InferedType::Tuple { items, span: _ } => items
                .iter()
                .map(|item| item.level(type_env))
                .max()
                .unwrap_or(0),
            InferedType::Record {
                fields,
                proto,
                span: _,
            } => fields
                .iter()
                .map(|(_, ty)| ty.level(type_env))
                .max()
                .unwrap_or(0)
                .max(proto.map(|proto| proto.level(type_env)).unwrap_or(0)),
            InferedType::List { item, span: _ } => item.level(type_env),
            InferedType::Ref {
                write,
                read,
                span: _,
            } => {
                let write = write.map(|write| write.level(type_env)).unwrap_or(0);
                let read = read.map(|read| read.level(type_env)).unwrap_or(0);
                write.max(read)
            }
        }
    }
}
