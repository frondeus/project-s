#![allow(dead_code)]

use core::WithID;
use std::{collections::BTreeMap, rc::Rc};

use crate::{
    ast::{ASTS, SExp, SExpId},
    diagnostics::Diagnostics,
    patterns::Pattern,
    source::Span,
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
        let prelude_span = Span::default();
        self.engine.number_def();
        self.engine.string_def();
        self.engine.bool_def();
        self.engine.keyword_def();

        self.envs.set(
            "list",
            core::Scheme::Polymorphic(Rc::new(move |this, _asts, diagnostics| {
                let pattern = Pattern::Single("args".into());
                this.envs.push();
                let pattern_bound = this.check_pattern(prelude_span.clone(), pattern);
                let ret_type = this.envs.get("args").unwrap();
                let ret_type = ret_type.as_mono().unwrap();
                let (_any_var, any_bound) = this.engine.var();
                let list_bound = this
                    .engine
                    .list_use(any_bound, 0, None, prelude_span.clone());
                this.engine.flow(ret_type, list_bound, diagnostics);
                this.envs.pop();

                this.engine
                    .func(pattern_bound, ret_type, prelude_span.clone())
            })),
        );

        self
    }

    fn null(&mut self, span: Span) -> core::Value {
        let def = self.engine.tuple_def(vec![], None);
        let list = self.engine.list(vec![], span);
        self.engine.new_def_edge(def, list);
        list
    }

    #[allow(clippy::result_large_err)]
    fn check(&mut self, asts: &ASTS, id: SExpId, diagnostics: &mut Diagnostics) -> core::Value {
        let sexp = asts.get(id);
        let span = sexp.span.clone();
        match &sexp.item {
            SExp::Number(_) => self.engine.number(span),
            SExp::String(_) => self.engine.string(span),
            SExp::Bool(_) => self.engine.bool(span),
            SExp::Keyword(_) => self.engine.keyword(span),
            SExp::Symbol(symbol) => match self.envs.get(symbol) {
                Some(scheme) => match scheme {
                    core::Scheme::Monomorphic(value) => *value,
                    core::Scheme::Polymorphic(f) => {
                        let f = f.clone();
                        f(self, asts, diagnostics)
                    }
                },
                None => {
                    diagnostics.add(span.clone(), format!("Undefined variable: {}", symbol));
                    self.engine.error(span)
                }
            },
            SExp::List(sexp_ids) => match sexp_ids.as_slice() {
                [] => self.engine.list(vec![], span),
                [first, condition, then_branch] if Self::is_symbol(asts, *first, "if") => {
                    let cond_type = self.check(asts, *condition, diagnostics);
                    let bool_span = self.span_of(*condition, asts);
                    let bound = self.engine.bool_use(bool_span);
                    self.engine.flow(cond_type, bound, diagnostics);

                    let then_type = self.check(asts, *then_branch, diagnostics);
                    let else_type = self.null(span.clone());

                    let (merged, merged_bound) = self.engine.var();
                    self.engine.flow(then_type, merged_bound, diagnostics);
                    self.engine.flow(else_type, merged_bound, diagnostics);

                    merged
                }
                [first, condition, then_branch, else_branch]
                    if Self::is_symbol(asts, *first, "if") =>
                {
                    let cond_type = self.check(asts, *condition, diagnostics);
                    let bool_span = self.span_of(*condition, asts);
                    let bound = self.engine.bool_use(bool_span);
                    self.engine.flow(cond_type, bound, diagnostics);

                    let then_type = self.check(asts, *then_branch, diagnostics);
                    let else_type = self.check(asts, *else_branch, diagnostics);

                    let (merged, merged_bound) = self.engine.var();
                    self.engine.flow(then_type, merged_bound, diagnostics);
                    self.engine.flow(else_type, merged_bound, diagnostics);

                    merged
                }
                [first, pattern, body] if Self::is_symbol(asts, *first, "fn") => {
                    let pattern = match Pattern::parse(*pattern, asts) {
                        Ok(pattern) => pattern,
                        Err(e) => {
                            diagnostics.add(span.clone(), format!("Unreadable pattern: {}", e));
                            return self.engine.error(span);
                        }
                    };

                    self.envs.push();
                    let pattern_bound = self.check_pattern(span.clone(), pattern);
                    // let pattern_must_be_list = self.engine.list_use

                    let body_type = self.check(asts, *body, diagnostics);
                    self.envs.pop();

                    let func_def = self.engine.func_def(span.clone());
                    let func = self.engine.func(pattern_bound, body_type, span);
                    self.engine.new_def_edge(func_def, func)
                }
                [first, args @ .., last] if Self::is_symbol(asts, *first, "do") => {
                    self.envs.push();
                    for arg in args {
                        self.check(asts, *arg, diagnostics);
                    }
                    let last_type = self.check(asts, *last, diagnostics);
                    self.envs.pop();
                    last_type
                }
                [first, pattern, value] if Self::is_symbol(asts, *first, "let") => {
                    let pattern = match Pattern::parse(*pattern, asts) {
                        Ok(pattern) => pattern,
                        Err(e) => {
                            diagnostics.add(span.clone(), format!("Unreadable pattern: {}", e));
                            return self.engine.error(span);
                        }
                    };

                    let value = *value;

                    self.polymorphic_check_pattern(span.clone(), pattern, value);

                    self.null(span)
                }
                [callee, args @ ..] => {
                    let callee_type = self.check(asts, *callee, diagnostics);
                    let args_types = args
                        .iter()
                        .map(|arg| self.check(asts, *arg, diagnostics))
                        .collect::<Vec<_>>();

                    let (ret_type, ret_bound) = self.engine.var();
                    // This will only work if the callee is a function
                    // We need to also be able to handle:
                    // * objects
                    // * constructors
                    // * macros
                    // * (in future) arrays
                    let bound = self.engine.func_use(args_types, ret_bound, span.clone());
                    self.engine.flow(callee_type, bound, diagnostics);
                    ret_type
                }
            },
            SExp::Error => self.engine.error(span),
        }
    }

    fn span_of(&self, sexp: SExpId, asts: &ASTS) -> Span {
        let sexp = asts.get(sexp);
        sexp.span.clone()
    }

    fn polymorphic_check_pattern(&mut self, span: Span, pattern: Pattern, value: SExpId) {
        match pattern {
            Pattern::Single(key) => {
                self.envs.set(
                    &key,
                    core::Scheme::Polymorphic(Rc::new(move |this, asts, diagnostics| {
                        this.check(asts, value, diagnostics)
                    })),
                );
            }
            Pattern::List(patterns) => {
                let mut bounds = Vec::new();
                for pattern in patterns {
                    let bound = self.check_pattern(span.clone(), pattern);
                    bounds.push(bound);
                }

                self.engine.tuple_use(bounds, span);
            }
            Pattern::Object(patterns) => {
                let mut bounds = Vec::new();
                for (key, pattern) in patterns {
                    let bound = self.check_pattern(span.clone(), pattern);
                    bounds.push((key, bound));
                }

                self.engine.obj_use(bounds, span);
            }
        }
    }

    fn check_pattern(&mut self, span: Span, pattern: Pattern) -> core::Use {
        match pattern {
            Pattern::Single(key) => {
                let (value, bound) = self.engine.var();
                self.envs.set(&key, core::Scheme::Monomorphic(value));
                bound
            }
            Pattern::List(patterns) => {
                let mut bounds = Vec::new();
                for pattern in patterns {
                    let bound = self.check_pattern(span.clone(), pattern);
                    bounds.push(bound);
                }

                self.engine.tuple_use(bounds, span)
            }
            Pattern::Object(patterns) => {
                let mut bounds = Vec::new();
                for (key, pattern) in patterns {
                    let bound = self.check_pattern(span.clone(), pattern);
                    bounds.push((key, bound));
                }

                self.engine.obj_use(bounds, span)
            }
        }
    }

    fn is_symbol(asts: &ASTS, sexp: SExpId, name: &str) -> bool {
        let sexp = asts.get(sexp);
        match &sexp.item {
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
                core::TypeNode::Def(_, _) => (),
                core::TypeNode::Value(vtype_head, _) => {
                    for to in vtype_head.ids() {
                        writeln!(buffer, "N{} -> N{} [color=blue, style=dotted];", id, to).unwrap();
                    }
                }
                core::TypeNode::Use(utype_head, _) => {
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
            let mut asts = ASTS::new();
            let ast = asts.parse(input, "<input>").expect("Failed to parse");

            let root = ast.root_id().unwrap();

            let mut env = TypeEnv::default().with_prelude();

            let mut diagnostics = Diagnostics::default();
            let infered = env.check(&asts, root, &mut diagnostics);
            if diagnostics.has_errors() {
                unsafe { std::env::set_var("NO_COLOR", "1") }
                return diagnostics.pretty_print();
            }

            env.to_string(infered)
        })
    }

    #[test]
    fn type_dot() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "graphviz", |input, _deps, _args| {
            let mut asts = ASTS::new();
            let ast = asts.parse(input, "<input>").expect("Failed to parse");
            let root = ast.root_id().unwrap();
            let mut env = TypeEnv::default().with_prelude();

            let mut diagnostics = Diagnostics::default();
            let root = env.check(&asts, root, &mut diagnostics);

            env.dot(root)
        })
    }
}
