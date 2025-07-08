use core::{Literal, WithID};
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    rc::Rc,
};

use builder::canonical_pair;
use canonical::{Canonical, CanonicalBuilder, Canonicalizer};
use itertools::Itertools;
use tree_sitter::Range;

use crate::{
    ast::{ASTS, SExp, SExpId},
    diagnostics::{Diagnostics, SExpDiag},
    modules::ModuleProvider,
    patterns::Pattern,
    source::{Span, WithSpan},
};

mod ascription;
mod builder;
mod canonical;
mod core;
mod prelude;
mod printing;
mod reachability;

pub struct TypeEnv {
    engine: core::TypeCheckerCore,
    envs: Envs,
    exprs: HashMap<SExpId, core::Value>,
    modules: Box<dyn ModuleProvider>,
}

impl TypeEnv {
    pub fn new(module: impl ModuleProvider) -> Self {
        Self {
            engine: core::TypeCheckerCore::default(),
            envs: Envs::default(),
            exprs: HashMap::default(),
            modules: Box::new(module),
        }
    }

    pub fn finish(self) -> Box<dyn ModuleProvider> {
        self.modules
    }

    fn null(&mut self, span: Span) -> core::Value {
        self.engine.tuple(vec![], span)
    }

    fn todo(&mut self, span: impl WithSpan) -> core::Value {
        let (ret_type, _) = self.engine.var(span);
        ret_type
    }

    fn assign_expr(&mut self, sexp_id: SExpId, id: core::Value) {
        self.exprs.entry(sexp_id).or_insert(id);
    }

    pub fn check(
        &mut self,
        asts: &mut ASTS,
        id: SExpId,
        diagnostics: &mut Diagnostics,
    ) -> core::Value {
        let type_ = self.check_inner(asts, id, diagnostics);
        self.assign_expr(id, type_);
        type_
    }

    pub fn get_ty_id(&self, id: SExpId) -> Option<core::Value> {
        self.exprs.get(&id).cloned()
    }

    pub fn get_canonical(&self, id: SExpId) -> Option<Canonical> {
        let id = self.get_ty_id(id)?;
        let (id, canonical) = Canonicalizer::default().canonicalize(id, &self.engine);
        Some(canonical.get(id).clone())
    }

    #[allow(clippy::result_large_err, clippy::unnecessary_to_owned)]
    fn check_inner(
        &mut self,
        asts: &mut ASTS,
        id: SExpId,
        diagnostics: &mut Diagnostics,
    ) -> core::Value {
        let sexp = asts.get(id);
        let span = sexp.span;
        match &**sexp {
            SExp::Number(n) => {
                let lit = self.engine.literal(Literal::Number(*n), span);
                let ty = self.engine.number_use(span);
                self.engine.flow(lit, ty, diagnostics);
                lit
            }
            SExp::String(s) => {
                let lit = self.engine.literal(Literal::String(s.clone()), span);
                let ty = self.engine.string_use(span);
                self.engine.flow(lit, ty, diagnostics);
                lit
            }

            SExp::Bool(b) => {
                let lit = self.engine.literal(Literal::Bool(*b), span);
                let ty = self.engine.bool_use(span);
                self.engine.flow(lit, ty, diagnostics);
                lit
            }
            SExp::Keyword(k) => {
                let lit = self.engine.literal(Literal::Keyword(k.clone()), span);
                let ty = self.engine.keyword_use(span);
                self.engine.flow(lit, ty, diagnostics);
                lit
            }
            SExp::Symbol(symbol) => match self.envs.get(symbol) {
                Some(scheme) => match scheme {
                    core::Scheme::Monomorphic(value) => *value,
                    core::Scheme::Polymorphic(f) => {
                        let f = f.clone();
                        f(self, asts, diagnostics)
                    }
                },
                None => {
                    diagnostics
                        .add_sexp(asts, id, format!("Undefined variable: {symbol}",))
                        .add_extra("Used here", Some(span));
                    self.engine.error(sexp)
                }
            },
            SExp::List(sexp_ids) => match sexp_ids.as_slice() {
                [] => self.engine.tuple(vec![], span),
                [first, ty, value] if Self::is_symbol(asts, *first, ":") => {
                    let mut builder = CanonicalBuilder::default();
                    let _ty_span = Self::span_of(*ty, asts);
                    let _ty = Self::parse_type(asts, *ty, &mut builder, diagnostics);
                    let (t_v, t_u) = canonical_pair(self, builder, _ty, _ty_span, diagnostics);
                    let value = self.check(asts, *value, diagnostics);
                    self.engine.flow(value, t_u, diagnostics);
                    t_v
                }
                &[first] if Self::is_symbol(asts, first, "module") => {
                    let Some(env) = self.envs.pop() else {
                        diagnostics.add(span, "No environment to create a module");
                        return self.engine.error(span);
                    };
                    self.engine.module(env, span)
                }
                &[first, path_id] if Self::is_symbol(asts, first, "import") => {
                    let path = self.check(asts, path_id, diagnostics);
                    let path_span = Self::span_of(path_id, asts);
                    let Some(path) = self.engine.find_value(path) else {
                        diagnostics
                            .add(span, "Importing a module requires a literal string")
                            .add_extra("Got", Some(path_span));
                        return self.engine.error(span);
                    };
                    let Some(path) = path.as_string_literal() else {
                        diagnostics
                            .add(span, "Importing a module requires a literal string")
                            .add_extra("Got", Some(path_span));
                        return self.engine.error(span);
                    };
                    let path = PathBuf::from(path);
                    let Some(module) = self.modules.get_module(&path) else {
                        diagnostics
                            .add(span, format!("Module not found: {}", path.display()))
                            .add_extra("Importing here", Some(span));
                        return self.engine.error(span);
                    };
                    let Some(source) = self.modules.get_source(module) else {
                        diagnostics
                            .add(span, format!("Module not found: {}", path.display()))
                            .add_extra("Importing here", Some(span));
                        return self.engine.error(span);
                    };
                    let Ok(root) = asts.parse(module, source) else {
                        diagnostics
                            .add(span, format!("Failed to parse module: {}", path.display()))
                            .add_extra("Parsing here", Some(span));
                        return self.engine.error(span);
                    };
                    let Some(root) = root.root_id() else {
                        diagnostics
                            .add(span, format!("Failed to parse module: {}", path.display()))
                            .add_extra("Parsing here", Some(span));
                        return self.engine.error(span);
                    };
                    let savepoint = self.envs.push();
                    let val = self.check(asts, root, diagnostics);
                    // We are not using here `envs.pop()` because
                    // when creating `(module)` it may already did the `pop()` operation.
                    // So instead we use restore() operation that works like a pop() if no pop operation
                    // was performed.
                    self.envs.restore(savepoint);

                    val
                }
                &[first, condition, then_branch] if Self::is_symbol(asts, first, "if") => {
                    let cond_type = self.check(asts, condition, diagnostics);
                    let bool_span = Self::span_of(condition, asts);
                    let bound = self.engine.bool_use(bool_span);
                    self.engine.flow(cond_type, bound, diagnostics);

                    let then_type = self.check(asts, then_branch, diagnostics);
                    let else_type = self.null(span);

                    let (merged, merged_bound) = self.engine.var(Self::span_of(then_branch, asts));
                    self.engine.flow(then_type, merged_bound, diagnostics);
                    self.engine.flow(else_type, merged_bound, diagnostics);

                    merged
                }
                &[first, condition, then_branch, else_branch]
                    if Self::is_symbol(asts, first, "if") =>
                {
                    let cond_type = self.check(asts, condition, diagnostics);
                    let bool_span = Self::span_of(condition, asts);
                    let bound = self.engine.bool_use(bool_span);
                    self.engine.flow(cond_type, bound, diagnostics);

                    let then_type = self.check(asts, then_branch, diagnostics);
                    let else_type = self.check(asts, else_branch, diagnostics);

                    let (merged, merged_bound) = self.engine.var(Self::span_of(then_branch, asts));
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
                                format!("Unreadable pattern: {e}",),
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
                                format!("Unreadable pattern: {e}"),
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
                [first, last] if Self::is_symbol(asts, *first, "do") => {
                    self.envs.push();
                    let body_type = self.check(asts, *last, diagnostics);
                    self.envs.pop();
                    body_type
                }
                [first, args @ .., last] if Self::is_symbol(asts, *first, "do") => {
                    self.envs.push();
                    let last = *last;
                    for arg in args.to_vec() {
                        self.check(asts, arg, diagnostics);
                    }
                    let last_type = self.check(asts, last, diagnostics);
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
                                format!("Unreadable pattern: {e}"),
                            );
                            return self.engine.error(Self::span_of(*pattern_id, asts));
                        }
                    };

                    let first = *first;
                    let value = *value;

                    self.polymorphic_check_pattern(pattern, value, asts, false, diagnostics);

                    self.null(Self::span_of(first, asts))
                }
                [first, bindings @ ..] if Self::is_symbols(asts, *first, &["let-rec", "let*"]) => {
                    let mut patterns = vec![];
                    let mut bindings = bindings.to_vec().into_iter();

                    while let Some(pattern) = bindings.next() {
                        let Some(value) = bindings.next() else {
                            diagnostics.add_sexp(asts, pattern, "Missing value");
                            return self.engine.error(span);
                        };

                        let parsed_pattern = match Pattern::parse(pattern, asts) {
                            Ok(p) => p,
                            Err(e) => {
                                diagnostics.add_sexp(
                                    asts,
                                    pattern,
                                    format!("Invalid pattern: {e}"),
                                );
                                return self.engine.error(Self::span_of(pattern, asts));
                            }
                        };

                        patterns.push((parsed_pattern, value));
                    }
                    let first = *first;

                    self.poly_rec_check_patterns(patterns, asts, diagnostics);

                    self.null(Self::span_of(first, asts))
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
                    for (key, value) in args.to_vec().into_iter().tuples() {
                        let key = match Self::as_keyword(asts, key) {
                            Some(key) => key,
                            None => {
                                diagnostics.add_sexp(asts, key, "Expected keyword");
                                return self.engine.error(Self::span_of(key, asts));
                            }
                        }
                        .to_string();
                        let value = self.check(asts, value, diagnostics);
                        fields.push((key, value));
                    }
                    self.engine.obj(fields, None, span)
                }
                [first, proto, args @ ..] if Self::is_symbol(asts, *first, "obj/extend") => {
                    let args = args.to_vec();
                    let proto = self.check(asts, *proto, diagnostics);
                    let mut fields = Vec::new();
                    for (key, value) in args.into_iter().tuples() {
                        let key = match Self::as_keyword(asts, key) {
                            Some(key) => key,
                            None => {
                                diagnostics.add_sexp(asts, key, "Expected keyword");
                                return self.engine.error(Self::span_of(key, asts));
                            }
                        }
                        .to_string();
                        let value = self.check(asts, value, diagnostics);
                        fields.push((key, value));
                    }
                    self.engine.obj(fields, Some(proto), span)
                }
                &[first, ref_mut, value_id] if Self::is_symbol(asts, first, "set") => {
                    let ref_mut = self.check(asts, ref_mut, diagnostics);
                    let value = self.check(asts, value_id, diagnostics);
                    let bound =
                        self.engine
                            .reference_use(Some(value), None, Self::span_of(value_id, asts));
                    self.engine.flow(ref_mut, bound, diagnostics);
                    value
                }
                [callee, args @ ..] => {
                    let args = args.to_vec();
                    let callee_type = self.check(asts, *callee, diagnostics);
                    let args_range = args
                        .iter()
                        .map(|arg| Self::span_of(*arg, asts).range)
                        .reduce(|a, b| Range {
                            start_byte: a.start_byte.min(b.start_byte),
                            start_point: a.start_point.min(b.start_point),
                            end_byte: a.end_byte.max(b.end_byte),
                            end_point: a.end_point.max(b.end_point),
                        })
                        .unwrap_or(span.range);

                    let args_span = Span {
                        range: args_range,
                        source_id: span.source_id,
                    };

                    let args_types = args
                        .iter()
                        .map(|arg| self.check(asts, *arg, diagnostics))
                        .collect::<Vec<_>>();

                    let (ret_type, ret_bound) = self.engine.var(span);

                    let bound = self
                        .engine
                        .application_use(args_types, ret_bound, args_span, span);
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

    #[allow(clippy::type_complexity)]
    fn poly_rec_check_patterns(
        &mut self,
        patterns: Vec<(Pattern, SExpId)>,
        asts: &mut ASTS,
        diagnostics: &mut Diagnostics,
    ) {
        let saved_patterns = patterns.clone();
        let f: Rc<dyn Fn(&mut TypeEnv, &mut ASTS, &mut Diagnostics, usize) -> core::Value> =
            Rc::new(
                move |this: &mut TypeEnv,
                      asts: &mut ASTS,
                      diagnostics: &mut Diagnostics,
                      idx: usize| {
                    let mut temp_vars = vec![];
                    for (pattern, _) in saved_patterns.iter() {
                        match pattern {
                            Pattern::Single(key, span, _id) => {
                                let temp_var = this.engine.var(span);
                                this.envs.set(key, core::Scheme::Monomorphic(temp_var.0));
                                temp_vars.push(temp_var);
                            }
                            _ => todo!(),
                        }
                    }
                    for ((_, expr), (_, bound)) in
                        saved_patterns.iter().zip(temp_vars.iter().copied())
                    {
                        let expr_type = this.check(asts, *expr, diagnostics);
                        this.engine.flow(expr_type, bound, diagnostics);
                    }

                    temp_vars[idx].0
                },
            );

        // f(self, asts, diagnostics, 0);

        for (i, (pat, value)) in patterns.into_iter().enumerate() {
            match pat {
                _ if !Self::is_expression_value(value, asts) => {
                    let value = f(self, asts, diagnostics, i);
                    let bound = self.check_pattern(pat);
                    self.engine.flow(value, bound, diagnostics);
                }
                Pattern::Single(key, _span, id) => {
                    let f = f.clone();
                    let value = f(self, asts, diagnostics, i);
                    let scheme = core::Scheme::Polymorphic(Rc::new(
                        move |this: &mut TypeEnv,
                              asts: &mut ASTS,
                              diagnostics: &mut Diagnostics| {
                            f(this, asts, diagnostics, i)
                        },
                    ));
                    self.assign_expr(id, value);
                    self.envs.set(&key, scheme);
                }
                _ => todo!(),
            }
        }
    }

    fn polymorphic_check_pattern(
        &mut self,
        pattern: Pattern,
        value: SExpId,
        asts: &mut ASTS,
        recursive: bool,
        diagnostics: &mut Diagnostics,
    ) {
        let bound = match pattern {
            // If its not a value, we cant generalize it so we treat is as monomorphic scheme.
            _ if !Self::is_expression_value(value, asts) => self.check_pattern(pattern),
            Pattern::Single(key, span, id) => {
                let inner_key = key.clone();
                let f =
                    move |this: &mut TypeEnv, asts: &mut ASTS, diagnostics: &mut Diagnostics| {
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
                    };
                let value = f(self, asts, diagnostics);
                self.assign_expr(id, value);

                self.envs.set(&key, core::Scheme::Polymorphic(Rc::new(f)));
                return;
            }
            Pattern::List(patterns, span, _id) => {
                let mut bounds = Vec::new();
                for pattern in patterns {
                    let bound = self.check_pattern(pattern);
                    bounds.push(bound);
                }

                self.engine.tuple_use(bounds, span)
            }
            Pattern::Object(patterns, span, _) => {
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
            Pattern::Single(key, span, id) => {
                let (value, bound) = self.engine.var(span);
                self.assign_expr(id, value);
                self.envs.set(&key, core::Scheme::Monomorphic(value));
                bound
            }
            Pattern::List(patterns, span, _) => {
                let mut bounds = Vec::new();
                for pattern in patterns {
                    let bound = self.check_pattern(pattern);
                    bounds.push(bound);
                }

                self.engine.tuple_use(bounds, span)
            }
            Pattern::Object(patterns, span, _) => {
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
                [first, ..] if Self::is_symbols(asts, *first, &["fn", "cl"]) => true,
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
                        writeln!(buffer, "N{id} -> N{to} [color=blue, style=dotted];").unwrap();
                    }
                }
                core::TypeNode::Use(utype_head, _) => {
                    for to in utype_head.ids() {
                        writeln!(buffer, "N{id} -> N{to} [color=red, style=dotted];").unwrap();
                    }
                }
            }
        }

        let graph = self.engine.reachability();
        for (id, _) in self.engine.iter() {
            for succ in graph.successors(id) {
                writeln!(buffer, "N{id} -> N{succ};").unwrap();
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

pub struct EnvSavePoint(usize);

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

    pub fn push(&mut self) -> EnvSavePoint {
        let point = EnvSavePoint(self.envs.len());
        self.envs.push(Env::default());
        point
    }

    pub fn pop(&mut self) -> Option<BTreeMap<String, core::Scheme>> {
        self.envs.pop().map(|env| env.vars)
    }

    pub fn restore(&mut self, point: EnvSavePoint) {
        self.envs.truncate(point.0);
    }

    // pub fn with<T>(&mut self, f: impl FnOnce() -> T) -> T {
    //     self.push();
    //     let result = f();
    //     self.pop();
    //     result
    // }
}

#[allow(clippy::print_stderr)]
#[cfg(test)]
mod tests {
    use crate::{
        ast::ASTS, macro_expansion::MacroExpansionPass, modules::MemoryModules, process_ast,
        s_std::prelude, source::Spanned,
    };

    use super::{canonical::Canonicalizer, *};

    #[test]
    fn type_() -> test_runner::Result {
        unsafe { std::env::set_var("NO_COLOR", "1") }
        test_runner::test_snapshots("docs/", &["s", ""], "type", |input, _deps, _args| {
            let mut asts = ASTS::new();
            let (modules, source_id) = MemoryModules::from_deps(input, _deps);
            let ast = asts
                .parse(source_id, modules.sources().get(source_id))
                .expect("Failed to parse");

            let root = ast.root_id().unwrap();

            let mut env = TypeEnv::new(modules).with_prelude();

            let prelude = prelude();
            let (root, mut diagnostics) = process_ast(&mut asts, root, &[prelude]);
            let infered = env.check(&mut asts, root, &mut diagnostics);
            if diagnostics.has_errors() {
                let modules = env.finish();
                return diagnostics.pretty_print(modules.sources());
            }

            env.to_string(infered)
        })
    }

    #[test]
    fn type_dot() -> test_runner::Result {
        test_runner::test_snapshots("docs/", &["", "s"], "graphviz", |input, deps, args| {
            let mut asts = ASTS::new();
            let (modules, source_id) = MemoryModules::from_deps(input, deps);
            let ast = asts
                .parse(source_id, modules.sources().get(source_id))
                .expect("Failed to parse");
            let root = ast.root_id().unwrap();
            let mut env = TypeEnv::new(modules).with_prelude();

            let mut diagnostics = Diagnostics::default();
            let prelude = prelude();

            let root = Spanned::new(root, ast.root().unwrap().span);
            let root = MacroExpansionPass::pass(&mut asts, root, &mut diagnostics, &[prelude]);
            let root = env.check(&mut asts, root.inner(), &mut diagnostics);

            if args.contains(&"canon") {
                let (canon_id, canonical) =
                    Canonicalizer::default().canonicalize(root, &env.engine);
                return canonical.dot(canon_id);
            }

            env.dot(root)
        })
    }
}
