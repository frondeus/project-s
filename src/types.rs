#![allow(dead_code)]

use core::WithID;
use std::{collections::BTreeMap, rc::Rc};

use crate::{
    ast::{ASTS, SExp, SExpId},
    patterns::Pattern,
};

mod core;
mod printing;
mod reachability;

#[derive(Default, Debug)]
pub struct TypeEnv {
    engine: core::TypeCheckerCore,
    envs: Envs,
}

impl TypeEnv {
    pub fn with_prelude(mut self) -> Self {
        self.engine.number();
        self.engine.string();
        self.engine.bool();
        self.engine.keyword();

        self.envs.set(
            "list",
            core::Scheme::Polymorphic(Rc::new(|this, _asts| {
                let pattern = Pattern::Single("args".into());
                this.envs.push();
                let pattern_bound = this.check_pattern(pattern);
                let ret_type = this.envs.get("args").unwrap();
                let ret_type = ret_type.as_mono().unwrap();
                let (_any_var, any_bound) = this.engine.var();
                let list_bound = this.engine.list_use(any_bound, 0, None);
                this.engine.flow(ret_type, list_bound).unwrap();
                this.envs.pop();

                let func = this.engine.func(pattern_bound, ret_type);
                Ok(func)
            })),
        );

        self
    }

    fn null(&mut self) -> core::Value {
        self.engine.list(vec![])
    }

    fn check(&mut self, asts: &ASTS, id: SExpId) -> core::Result<core::Value> {
        let sexp = asts.get(id);
        match sexp {
            SExp::Number(_) => Ok(self.engine.number()),
            SExp::String(_) => Ok(self.engine.string()),
            SExp::Bool(_) => Ok(self.engine.bool()),
            SExp::Keyword(_) => Ok(self.engine.keyword()),
            SExp::Symbol(symbol) => match self.envs.get(symbol) {
                Some(scheme) => match scheme {
                    core::Scheme::Monomorphic(value) => Ok(*value),
                    core::Scheme::Polymorphic(f) => {
                        let f = f.clone();
                        f(self, asts)
                    }
                },
                None => Err(core::TypeError::UndefinedVariable(symbol.to_owned())),
            },
            SExp::List(sexp_ids) => match sexp_ids.as_slice() {
                [] => Ok(self.engine.list(vec![])),
                [first, condition, then_branch] if Self::is_symbol(asts, *first, "if") => {
                    let cond_type = self.check(asts, *condition)?;
                    let bound = self.engine.bool_use();
                    self.engine.flow(cond_type, bound)?;

                    let then_type = self.check(asts, *then_branch)?;
                    let else_type = self.null();

                    let (merged, merged_bound) = self.engine.var();
                    self.engine.flow(then_type, merged_bound)?;
                    self.engine.flow(else_type, merged_bound)?;

                    Ok(merged)
                }
                [first, condition, then_branch, else_branch]
                    if Self::is_symbol(asts, *first, "if") =>
                {
                    let cond_type = self.check(asts, *condition)?;
                    let bound = self.engine.bool_use();
                    self.engine.flow(cond_type, bound)?;

                    let then_type = self.check(asts, *then_branch)?;
                    let else_type = self.check(asts, *else_branch)?;

                    let (merged, merged_bound) = self.engine.var();
                    self.engine.flow(then_type, merged_bound)?;
                    self.engine.flow(else_type, merged_bound)?;

                    Ok(merged)
                }
                [first, pattern, body] if Self::is_symbol(asts, *first, "fn") => {
                    let pattern = Pattern::parse(*pattern, asts)
                        .map_err(core::TypeError::UnreadablePattern)?;

                    self.envs.push();
                    let pattern_bound = self.check_pattern(pattern);
                    // let pattern_must_be_list = self.engine.list_use

                    let body_type = self.check(asts, *body);
                    self.envs.pop();
                    let body_type = body_type?;

                    Ok(self.engine.func(pattern_bound, body_type))
                }
                [first, args @ .., last] if Self::is_symbol(asts, *first, "do") => {
                    self.envs.push();
                    for arg in args {
                        if let Err(e) = self.check(asts, *arg) {
                            self.envs.pop();
                            return Err(e);
                        }
                    }
                    let last_type = self.check(asts, *last);
                    self.envs.pop();
                    last_type
                }
                [first, pattern, value] if Self::is_symbol(asts, *first, "let") => {
                    let pattern = Pattern::parse(*pattern, asts)
                        .map_err(core::TypeError::UnreadablePattern)?;

                    let value = *value;

                    self.polymorphic_check_pattern(pattern, value);

                    Ok(self.null())
                }
                [callee, args @ ..] => {
                    let callee_type = self.check(asts, *callee)?;
                    let args_types = args
                        .iter()
                        .map(|arg| self.check(asts, *arg))
                        .collect::<core::Result<Vec<_>>>()?;

                    let (ret_type, ret_bound) = self.engine.var();
                    // This will only work if the callee is a function
                    // We need to also be able to handle:
                    // * objects
                    // * constructors
                    // * macros
                    // * (in future) arrays
                    let bound = self.engine.func_use(args_types, ret_bound);
                    self.engine.flow(callee_type, bound)?;
                    Ok(ret_type)
                }
            },
            SExp::Error => Ok(self.engine.error()),
        }
    }

    fn polymorphic_check_pattern(&mut self, pattern: Pattern, value: SExpId) {
        match pattern {
            Pattern::Single(key) => {
                self.envs.set(
                    &key,
                    core::Scheme::Polymorphic(Rc::new(move |this, asts| {
                        let value_type = this.check(asts, value)?;
                        Ok(value_type)
                    })),
                );
            }
            Pattern::List(patterns) => {
                let mut bounds = Vec::new();
                for pattern in patterns {
                    let bound = self.check_pattern(pattern);
                    bounds.push(bound);
                }

                self.engine.tuple_use(bounds);
            }
            Pattern::Object(patterns) => {
                let mut bounds = Vec::new();
                for (key, pattern) in patterns {
                    let bound = self.check_pattern(pattern);
                    bounds.push((key, bound));
                }

                self.engine.obj_use(bounds);
            }
        }
    }

    fn check_pattern(&mut self, pattern: Pattern) -> core::Use {
        match pattern {
            Pattern::Single(key) => {
                let (value, bound) = self.engine.var();
                self.envs.set(&key, core::Scheme::Monomorphic(value));
                bound
            }
            Pattern::List(patterns) => {
                let mut bounds = Vec::new();
                for pattern in patterns {
                    let bound = self.check_pattern(pattern);
                    bounds.push(bound);
                }

                self.engine.tuple_use(bounds)
            }
            Pattern::Object(patterns) => {
                let mut bounds = Vec::new();
                for (key, pattern) in patterns {
                    let bound = self.check_pattern(pattern);
                    bounds.push((key, bound));
                }

                self.engine.obj_use(bounds)
            }
        }
    }

    fn is_symbol(asts: &ASTS, sexp: SExpId, name: &str) -> bool {
        let sexp = asts.get(sexp);
        match sexp {
            SExp::Symbol(symbol) => symbol == name,
            _ => false,
        }
    }

    // ------------ DEBUG ---------------
    pub fn dot(&self, root: core::Value) -> String {
        use std::fmt::Write;
        let mut buffer = String::new();
        writeln!(buffer, "digraph G {{").unwrap();
        for (id, node) in self.engine.iter() {
            writeln!(buffer, "N{id} [label=\"{id}: {node:?}\"];").unwrap();
        }

        writeln!(buffer, "START -> N{}", root.id()).unwrap();

        for (id, node) in self.engine.iter() {
            match node {
                core::TypeNode::Var => (),
                core::TypeNode::Value(vtype_head) => {
                    for to in vtype_head.ids() {
                        writeln!(buffer, "N{} -> N{} [color=blue, style=dotted];", id, to).unwrap();
                    }
                }
                core::TypeNode::Use(utype_head) => {
                    for to in utype_head.ids() {
                        writeln!(buffer, "N{} -> N{} [color=red, style=dotted];", id, to).unwrap();
                    }
                }
            }
        }

        let graph = self.engine.reachability();
        for (id, _) in self.engine.iter() {
            for succ in graph.successors(id) {
                writeln!(buffer, "N{} -> N{};", id, succ).unwrap();
            }
        }

        writeln!(buffer, "}}").unwrap();

        buffer
    }
}

#[derive(Default, Debug)]
struct Env {
    vars: BTreeMap<String, core::Scheme>,
}

#[derive(Debug)]
struct Envs {
    envs: Vec<Env>,
}

impl Default for Envs {
    fn default() -> Self {
        Self::new()
    }
}

impl Envs {
    pub fn new() -> Self {
        Self {
            envs: vec![Env::default()],
        }
    }

    pub fn set(&mut self, name: &str, value: core::Scheme) {
        self.envs
            .last_mut()
            .unwrap()
            .vars
            .insert(name.to_string(), value);
    }

    pub fn get(&self, name: &str) -> Option<&core::Scheme> {
        self.envs.iter().rev().find_map(|env| env.vars.get(name))
    }

    pub fn push(&mut self) {
        self.envs.push(Env::default());
    }

    pub fn pop(&mut self) -> Option<BTreeMap<String, core::Scheme>> {
        self.envs.pop().map(|env| env.vars)
    }

    pub fn with<T>(&mut self, f: impl FnOnce() -> T) -> T {
        self.push();
        let result = f();
        self.pop();
        result
    }
}

#[allow(clippy::print_stderr)]
#[cfg(test)]
mod tests {
    use crate::ast::ASTS;

    use super::*;

    #[test]
    fn type_() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "type", |input, _deps, _args| {
            eprintln!("Before parsing");
            let mut asts = ASTS::new();
            let ast = asts.parse(input).expect("Failed to parse");
            eprintln!("After parsing");

            let root = ast.root_id().unwrap();

            eprintln!("Before prelude");

            let mut env = TypeEnv::default().with_prelude();

            eprintln!("After prelude");

            eprintln!("Before infering");

            let infered = match env.check(&asts, root) {
                Ok(infered) => infered,
                Err(e) => return format!("ERROR: {:?}", e),
            };

            eprintln!("After infering");

            env.to_string(infered)
        })
    }

    #[test]
    fn type_dot() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "graphviz", |input, _deps, _args| {
            let mut asts = ASTS::new();
            let ast = asts.parse(input).expect("Failed to parse");
            let root = ast.root_id().unwrap();
            let mut env = TypeEnv::default().with_prelude();

            let root = env.check(&asts, root).unwrap();

            env.dot(root)
        })
    }
}
