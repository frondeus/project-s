#![allow(dead_code)]

use core::WithID;
use std::{collections::BTreeMap, rc::Rc};

use builder::{
    TypeBuilder,
    canon::{CanonBuilder, keyword, number},
    canonical_pair, u_canonical,
};
use canonical::{Canonical, CanonicalBuilder};
use itertools::Itertools;

use crate::{
    ast::{ASTS, SExp, SExpId},
    diagnostics::{Diagnostics, SExpDiag},
    patterns::Pattern,
    source::{Sources, Span, WithSpan},
};

mod ascription;
mod builder;
mod canonical;
mod core;
mod printing;
mod reachability;

#[derive(Default, Debug)]
pub struct TypeEnv {
    engine: core::TypeCheckerCore,
    envs: Envs,
}

impl TypeEnv {
    pub fn with_prelude(self, sources: &mut Sources) -> Self {
        let mut env = self;
        use builder::canon::*;

        let builtin = sources.add("<builtin>", "");
        let builtin = Span::new_empty(builtin);

        env.with_poly("list", || func(list(any(0)), list(any(0))), builtin);
        env.with_poly("tuple", || func(any(0), any(0)), builtin);

        env.with_mono("+", func(list(number()), number()), builtin);
        env.with_mono("-", func(list(number()), number()), builtin);
        env.with_mono(">", func((number(), number()), bool()), builtin);
        env.with_poly("print", || func(list(any(None)), number()), builtin);

        let empty_struct = Canonical::Struct { fields: vec![] };
        let empty_struct_ref = reference(Some(empty_struct.clone()), Some(empty_struct));

        env.with_mono(
            "obj/insert",
            func((empty_struct_ref, keyword(), any(None)), ()),
            builtin,
        );
        // TODO: that is not fully correct. We want to have type
        // (Con<T0>) -> T0
        //    | (T0) -> T0
        env.with_mono("obj/construct-or", func((any(0),), any(0)), builtin);
        env
    }

    fn with_poly<F, C>(&mut self, name: &str, value: F, span: Span)
    where
        F: 'static + Fn() -> C,
        C: CanonBuilder,
    {
        use builder::v_canonical;
        self.envs.set(
            name,
            core::Scheme::Polymorphic(Rc::new(move |env, _asts, diagnostics| {
                v_canonical(value(), span).build(env, diagnostics)
            })),
        );
    }
    fn with_mono(&mut self, name: &str, value: impl CanonBuilder, span: Span) {
        use builder::v_canonical;
        let value = v_canonical(value, span).build(self, &mut Diagnostics::default());
        self.envs.set(name, core::Scheme::Monomorphic(value));
    }

    fn null(&mut self, span: Span) -> core::Value {
        self.engine.tuple(vec![], span)
    }

    fn todo(&mut self, span: impl WithSpan) -> core::Value {
        let (ret_type, _) = self.engine.var(span);
        ret_type
    }

    #[allow(clippy::result_large_err)]
    pub fn check(&mut self, asts: &ASTS, id: SExpId, diagnostics: &mut Diagnostics) -> core::Value {
        let sexp = asts.get(id);
        let span = sexp.span;
        match &**sexp {
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
                    diagnostics.add_sexp(asts, id, format!("Undefined variable: {}", symbol));
                    self.engine.error(sexp)
                }
            },
            SExp::List(sexp_ids) => match sexp_ids.as_slice() {
                [] => self.engine.tuple(vec![], span),
                [first, ty, value] if Self::is_symbol(asts, *first, ":") => {
                    let mut builder = CanonicalBuilder::default();
                    let _ty = Self::parse_type(asts, *ty, &mut builder, diagnostics);
                    let (t_v, t_u) = canonical_pair(self, builder, _ty, span, diagnostics);
                    let value = self.check(asts, *value, diagnostics);
                    self.engine.flow(value, t_u, diagnostics);
                    t_v
                }
                [first, condition, then_branch] if Self::is_symbol(asts, *first, "if") => {
                    let cond_type = self.check(asts, *condition, diagnostics);
                    let bool_span = Self::span_of(*condition, asts);
                    let bound = self.engine.bool_use(bool_span);
                    self.engine.flow(cond_type, bound, diagnostics);

                    let then_type = self.check(asts, *then_branch, diagnostics);
                    let else_type = self.null(span);

                    let (merged, merged_bound) = self.engine.var(Self::span_of(*then_branch, asts));
                    self.engine.flow(then_type, merged_bound, diagnostics);
                    self.engine.flow(else_type, merged_bound, diagnostics);

                    merged
                }
                [first, condition, then_branch, else_branch]
                    if Self::is_symbol(asts, *first, "if") =>
                {
                    let cond_type = self.check(asts, *condition, diagnostics);
                    let bool_span = Self::span_of(*condition, asts);
                    let bound = self.engine.bool_use(bool_span);
                    self.engine.flow(cond_type, bound, diagnostics);

                    let then_type = self.check(asts, *then_branch, diagnostics);
                    let else_type = self.check(asts, *else_branch, diagnostics);

                    let (merged, merged_bound) = self.engine.var(Self::span_of(*then_branch, asts));
                    self.engine.flow(then_type, merged_bound, diagnostics);
                    self.engine.flow(else_type, merged_bound, diagnostics);

                    merged
                }
                [first, pattern_id, body] if Self::is_symbol(asts, *first, "fn") => {
                    let pattern = match Pattern::parse(*pattern_id, asts) {
                        Ok(pattern) => pattern,
                        Err(e) => {
                            diagnostics.add_sexp(
                                asts,
                                *pattern_id,
                                format!("Unreadable pattern: {}", e),
                            );
                            return self.engine.error(Self::span_of(*pattern_id, asts));
                        }
                    };

                    self.envs.push();
                    let pattern_bound = self.check_pattern(pattern);
                    // let pattern_must_be_list = self.engine.list_use

                    let body_type = self.check(asts, *body, diagnostics);
                    self.envs.pop();

                    self.engine.func(pattern_bound, body_type, span)
                }
                [first, pattern_id, _captured, body] if Self::is_symbol(asts, *first, "cl") => {
                    //For now lets ignore captured...
                    let pattern = match Pattern::parse(*pattern_id, asts) {
                        Ok(pattern) => pattern,
                        Err(e) => {
                            diagnostics.add_sexp(
                                asts,
                                *pattern_id,
                                format!("Unreadable pattern: {}", e),
                            );
                            return self.engine.error(Self::span_of(*pattern_id, asts));
                        }
                    };

                    self.envs.push();
                    let pattern_bound = self.check_pattern(pattern);
                    // let pattern_must_be_list = self.engine.list_use

                    let body_type = self.check(asts, *body, diagnostics);
                    self.envs.pop();

                    self.engine.func(pattern_bound, body_type, span)
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
                [first, pattern_id, value] if Self::is_symbol(asts, *first, "let") => {
                    let pattern = match Pattern::parse(*pattern_id, asts) {
                        Ok(pattern) => pattern,
                        Err(e) => {
                            diagnostics.add_sexp(
                                asts,
                                *pattern_id,
                                format!("Unreadable pattern: {}", e),
                            );
                            return self.engine.error(Self::span_of(*pattern_id, asts));
                        }
                    };

                    let value = *value;

                    self.polymorphic_check_pattern(pattern, value, asts, false, diagnostics);

                    self.null(Self::span_of(*first, asts))
                }
                [first, pattern_id, value] if Self::is_symbol(asts, *first, "let-rec") => {
                    let pattern = match Pattern::parse(*pattern_id, asts) {
                        Ok(pattern) => pattern,
                        Err(e) => {
                            diagnostics.add_sexp(
                                asts,
                                *pattern_id,
                                format!("Unreadable pattern: {}", e),
                            );
                            return self.engine.error(Self::span_of(*pattern_id, asts));
                        }
                    };

                    let value = *value;

                    self.polymorphic_check_pattern(pattern, value, asts, true, diagnostics);

                    self.null(Self::span_of(*first, asts))
                }
                [first, _err] if Self::is_symbol(asts, *first, "error") => self.engine.error(span),
                [first, _captured, rest] if Self::is_symbol(asts, *first, "thunk") => {
                    self.check(asts, *rest, diagnostics)
                }
                [first, ..] if Self::is_symbol(asts, *first, "macro") => self.todo(span),
                [first, ..] if Self::is_symbols(asts, *first, &["quote", "quasiquote"]) => {
                    self.todo(span)
                }
                [first, value] if Self::is_symbol(asts, *first, "ref") => {
                    let value_type = self.check(asts, *value, diagnostics);
                    let (read, write) = self.engine.var(span);
                    self.engine.flow(value_type, write, diagnostics);
                    self.engine.reference(Some(write), Some(read), span)
                }
                [first, args @ ..] if Self::is_symbol(asts, *first, "obj/plain") => {
                    let mut fields = Vec::new();
                    for (key, value) in args.iter().tuples() {
                        let key = match Self::as_keyword(asts, *key) {
                            Some(key) => key,
                            None => {
                                diagnostics.add_sexp(asts, *key, "Expected keyword");
                                return self.engine.error(Self::span_of(*key, asts));
                            }
                        };
                        let value = self.check(asts, *value, diagnostics);
                        fields.push((key.to_string(), value));
                    }
                    self.engine.obj(fields, None, span)
                }
                [first, proto, args @ ..] if Self::is_symbol(asts, *first, "obj/extend") => {
                    let proto = self.check(asts, *proto, diagnostics);
                    let mut fields = Vec::new();
                    for (key, value) in args.iter().tuples() {
                        let key = match Self::as_keyword(asts, *key) {
                            Some(key) => key,
                            None => {
                                diagnostics.add_sexp(asts, *key, "Expected keyword");
                                return self.engine.error(Self::span_of(*key, asts));
                            }
                        };
                        let value = self.check(asts, *value, diagnostics);
                        fields.push((key.to_string(), value));
                    }
                    self.engine.obj(fields, Some(proto), span)
                }
                [first, ref_mut, value_id] if Self::is_symbol(asts, *first, "set") => {
                    let ref_mut = self.check(asts, *ref_mut, diagnostics);
                    let value = self.check(asts, *value_id, diagnostics);
                    let bound = self.engine.reference_use(
                        Some(value),
                        None,
                        Self::span_of(*value_id, asts),
                    );
                    self.engine.flow(ref_mut, bound, diagnostics);
                    value
                }
                [callee, args @ ..] => {
                    let callee_type = self.check(asts, *callee, diagnostics);
                    let args_types = args
                        .iter()
                        .map(|arg| self.check(asts, *arg, diagnostics))
                        .collect::<Vec<_>>();

                    let (ret_type, ret_bound) = self.engine.var(span);

                    let index_use = u_canonical((number(),), span).build(self, diagnostics);
                    let field_use = u_canonical((keyword(),), span).build(self, diagnostics);

                    let first_arg_index = args
                        .first()
                        .and_then(|arg| match &**asts.get(*arg) {
                            SExp::Number(idx) => Some(idx),
                            _ => None,
                        })
                        .map(|idx| *idx as usize);

                    let first_arg_keyword = args.first().and_then(|arg| match &**asts.get(*arg) {
                        SExp::Keyword(s) => Some(s.clone()),
                        _ => None,
                    });

                    let bound = self.engine.application_use(
                        args_types,
                        ret_bound,
                        (first_arg_keyword, field_use),
                        (first_arg_index, index_use),
                        span,
                        span,
                    );
                    self.engine.flow(callee_type, bound, diagnostics);
                    ret_type
                }
            },
            SExp::Error => self.engine.error(span),
        }
    }

    fn span_of(sexp: SExpId, asts: &ASTS) -> Span {
        let sexp = asts.get(sexp);
        sexp.span
    }

    fn polymorphic_check_pattern(
        &mut self,
        pattern: Pattern,
        value: SExpId,
        asts: &ASTS,
        recursive: bool,
        diagnostics: &mut Diagnostics,
    ) {
        let bound = match pattern {
            // If its not a value, we cant generalize it so we treat is as monomorphic scheme.
            _ if !Self::is_expression_value(value, asts) => self.check_pattern(pattern),
            Pattern::Single(key, span) => {
                let inner_key = key.clone();
                self.envs.set(
                    &key,
                    core::Scheme::Polymorphic(Rc::new(move |this, asts, diagnostics| {
                        if !recursive {
                            this.check(asts, value, diagnostics)
                        } else {
                            let (temp_type, temp_bound) = this.engine.var(span);
                            this.envs
                                .set(&inner_key, core::Scheme::Monomorphic(temp_type));

                            let var_type = this.check(asts, value, diagnostics);
                            this.engine.flow(var_type, temp_bound, diagnostics);
                            temp_type
                        }
                    })),
                );
                return;
            }
            Pattern::List(patterns, span) => {
                let mut bounds = Vec::new();
                for pattern in patterns {
                    let bound = self.check_pattern(pattern);
                    bounds.push(bound);
                }

                self.engine.tuple_use(bounds, span)
            }
            Pattern::Object(patterns, span) => {
                let mut bounds = Vec::new();
                for (key, pattern) in patterns {
                    let bound = self.check_pattern(pattern);
                    bounds.push((key, bound));
                }

                self.engine.obj_use(bounds, span)
            }
        };
        let value = self.check(asts, value, diagnostics);
        self.engine.flow(value, bound, diagnostics);
    }

    fn check_pattern(&mut self, pattern: Pattern) -> core::Use {
        match pattern {
            Pattern::Single(key, span) => {
                let (value, bound) = self.engine.var(span);
                self.envs.set(&key, core::Scheme::Monomorphic(value));
                bound
            }
            Pattern::List(patterns, span) => {
                let mut bounds = Vec::new();
                for pattern in patterns {
                    let bound = self.check_pattern(pattern);
                    bounds.push(bound);
                }

                self.engine.tuple_use(bounds, span)
            }
            Pattern::Object(patterns, span) => {
                let mut bounds = Vec::new();
                for (key, pattern) in patterns {
                    let bound = self.check_pattern(pattern);
                    bounds.push((key, bound));
                }

                self.engine.obj_use(bounds, span)
            }
        }
    }

    /// Is the expression a value in the context of "value restriciton" used to determine
    /// if we can generalize a type returned by this expression or not.
    ///
    /// For now, we say that the expression is a value if it does not contain any function
    /// calls.
    fn is_expression_value(sexp: SExpId, asts: &ASTS) -> bool {
        match &**asts.get(sexp) {
            SExp::Number(_)
            | SExp::String(_)
            | SExp::Bool(_)
            | SExp::Symbol(_)
            | SExp::Error
            | SExp::Keyword(_) => true,
            SExp::List(sexp_ids) => match sexp_ids.as_slice() {
                [first, ..] if Self::is_symbol(asts, *first, "fn") => true,
                [first, rest @ ..] if Self::is_symbols(asts, *first, &["do", "if", "let"]) => rest
                    .iter()
                    .all(|sexp_id| Self::is_expression_value(*sexp_id, asts)),
                // Its a function call
                _ => false,
            },
        }
    }

    fn is_symbols(asts: &ASTS, sexp: SExpId, names: &[&str]) -> bool {
        let sexp = asts.get(sexp);
        match &**sexp {
            SExp::Symbol(symbol) => names.contains(&symbol.as_str()),
            _ => false,
        }
    }

    fn is_symbol(asts: &ASTS, sexp: SExpId, name: &str) -> bool {
        let sexp = asts.get(sexp);
        match &**sexp {
            SExp::Symbol(symbol) => symbol == name,
            _ => false,
        }
    }

    fn as_keyword(asts: &ASTS, sexp: SExpId) -> Option<&str> {
        let sexp = asts.get(sexp);
        match &**sexp {
            SExp::Keyword(s) => Some(s),
            _ => None,
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
                core::TypeNode::Var(_) => (),
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
    use crate::{
        ast::ASTS,
        macro_expansion::MacroExpansionPass,
        s_std::prelude,
        source::{Sources, Spanned},
    };

    use super::{canonical::Canonicalizer, *};

    #[test]
    fn type_() -> test_runner::Result {
        unsafe { std::env::set_var("NO_COLOR", "1") }
        test_runner::test_snapshots("docs/", "type", |input, _deps, _args| {
            let mut asts = ASTS::new();
            let (mut sources, source_id) = Sources::single("<input>", input);
            let ast = asts
                .parse(source_id, sources.get(source_id))
                .expect("Failed to parse");

            let root = ast.root_id().unwrap();

            let mut env = TypeEnv::default().with_prelude(&mut sources);

            let mut diagnostics = Diagnostics::default();
            let prelude = prelude();
            let root = Spanned::new(root, ast.root().unwrap().span);
            let root = MacroExpansionPass::pass(&mut asts, root, &mut diagnostics, &[prelude]);
            let infered = env.check(&asts, root.inner(), &mut diagnostics);
            if diagnostics.has_errors() {
                return diagnostics.pretty_print(&sources);
            }

            env.to_string(infered)
        })
    }

    #[test]
    fn type_dot() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "graphviz", |input, _deps, args| {
            let mut asts = ASTS::new();
            let (mut sources, source_id) = Sources::single("<input>", input);
            let ast = asts
                .parse(source_id, sources.get(source_id))
                .expect("Failed to parse");
            let root = ast.root_id().unwrap();
            let mut env = TypeEnv::default().with_prelude(&mut sources);

            let mut diagnostics = Diagnostics::default();
            let prelude = prelude();

            let root = Spanned::new(root, ast.root().unwrap().span);
            let root = MacroExpansionPass::pass(&mut asts, root, &mut diagnostics, &[prelude]);
            let root = env.check(&asts, root.inner(), &mut diagnostics);

            if args.contains(&"canon") {
                let (canon_id, canonical) =
                    Canonicalizer::default().canonicalize(root, &env.engine);
                return canonical.dot(canon_id);
            }

            env.dot(root)
        })
    }
}
