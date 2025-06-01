#![allow(dead_code)]

use std::collections::HashMap;

use crate::ast::{AST, SExp, SExpId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Type {
    Number,
    String,
    Bool,
    Symbol,
    Error,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(usize);

#[derive(Default)]
pub struct TypeEnv {
    types: Vec<Type>,
    exprs: HashMap<SExpId, TypeId>,
}

impl TypeEnv {
    pub fn add(&mut self, ty: Type) -> TypeId {
        let id = self.types.len();
        self.types.push(ty);
        TypeId(id)
    }

    pub fn get(&self, id: TypeId) -> &Type {
        &self.types[id.0]
    }

    pub fn infer(&mut self, ast: &AST, id: SExpId) -> TypeId {
        let sexp = ast.get(id);
        match sexp {
            SExp::Number(_) => self.add(Type::Number),
            SExp::String(_) => self.add(Type::String),
            SExp::Symbol(_) => self.add(Type::Symbol),
            SExp::Bool(_) => self.add(Type::Bool),
            SExp::List(_) => todo!(),
            SExp::Error => self.add(Type::Error),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::ASTS;

    use super::*;

    #[test]
    fn integration() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "type", |input, _deps| {
            let mut asts = ASTS::new();
            let ast = asts.parse(input).expect("Failed to parse");
            let mut env = TypeEnv::default();
            let infered = env.infer(ast, ast.root_id().unwrap());
            let result = env.get(infered);

            format!("{:?}", result)
        })
    }
}
