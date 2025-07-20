use std::path::PathBuf;

use crate::modules::ModuleProvider;

use super::*;

#[derive(Copy, Clone)]
enum TypeOrRest {
    Type,
    Rest,
}

#[derive(Copy, Clone)]
enum PartOf {
    ListRest,
    ObjectRest,
    Nothing,
    List,
    Object,
}

impl PartOf {
    fn into_rest(self) -> Self {
        match self {
            PartOf::List => PartOf::ListRest,
            PartOf::Object => PartOf::ObjectRest,
            s => s,
        }
    }
}

impl TypeEnv {
    pub fn type_term(
        &mut self,
        asts: &mut ASTS,
        id: SExpId,
        diagnostics: &mut Diagnostics,
        modules: &mut dyn ModuleProvider,
        level: usize,
    ) -> InferedTypeId {
        let ty = self.type_term_inner(asts, id, diagnostics, modules, level);
        self.add_sexp(asts, id, ty);
        ty
    }

    fn type_term_inner(
        &mut self,
        asts: &mut ASTS,
        id: SExpId,
        diagnostics: &mut Diagnostics,
        modules: &mut dyn ModuleProvider,
        level: usize,
    ) -> InferedTypeId {
        let sexp = asts.get(id);
        let span = sexp.span;
        tracing::trace!("Infering `{}` level {}", asts.fmt(id), level);
        match &**sexp {
            &SExp::Number(n) => {
                let lit = self.literal(Literal::Number(n), span);
                let number = self.primitive(Self::NUMBER, span);

                self.constrain(lit, number, diagnostics);
                lit
            }
            SExp::String(s) => {
                let lit = self.literal(Literal::String(s.clone()), span);
                let string = self.primitive(Self::STRING, span);

                self.constrain(lit, string, diagnostics);
                lit
            }
            &SExp::Bool(b) => {
                let lit = self.literal(Literal::Bool(b), span);
                let bool = self.primitive(Self::BOOLEAN, span);

                self.constrain(lit, bool, diagnostics);
                lit
            }
            SExp::Symbol(s) => match self.envs.get(s).copied() {
                Some(ty) => {
                    tracing::warn!("Found symbol: {ty:?}");
                    ty.instantiate(self, level)
                }
                None => {
                    diagnostics
                        .add_sexp(asts, id, format!("Undefined variable: {s}"))
                        .add_extra("Used here", Some(span));
                    self.error(span)
                }
            },
            SExp::Keyword(key) => {
                let lit = self.literal(Literal::Keyword(key.clone()), span);
                let keyword = self.primitive(Self::KEYWORD, span);

                self.constrain(lit, keyword, diagnostics);
                lit
            }
            SExp::List(sexp_ids) => match *sexp_ids.as_slice() {
                [] => self.unit(span),
                [first, ty, value] if Self::is_symbol(asts, first, ":") => {
                    let ty = self.ascribe(asts, ty, diagnostics, &mut Default::default(), level);
                    let value_ty = self.type_term(asts, value, diagnostics, modules, level);
                    self.add_sexp(asts, value, ty);
                    self.constrain(value_ty, ty, diagnostics);
                    ty
                }
                [first] if Self::is_symbol(asts, first, "module") => {
                    self.module(Default::default(), span)
                }
                [first, ref exprs @ ..] if Self::is_symbol(asts, first, "module") => {
                    self.envs.push();
                    for e in exprs.to_vec() {
                        self.type_term(asts, e, diagnostics, modules, level);
                    }
                    let env = self.envs.pop().unwrap();

                    self.module(env, span)
                }
                [first, path_id] if Self::is_symbol(asts, first, "import") => {
                    let path = self.type_term(asts, path_id, diagnostics, modules, level);
                    let path_span = Self::span_of(path_id, asts);
                    let Some(path) =
                        self.find_in_predecessors(path, InferedType::as_string_literal)
                    else {
                        diagnostics
                            .add(span, "Expected string literal")
                            .add_extra("Got", Some(path_span));
                        return self.error(span);
                    };
                    let path = PathBuf::from(path);
                    let Some((module, source)) = modules.get_module(&path).and_then(|module| {
                        let source = modules.get_source(module)?;
                        Some((module, source))
                    }) else {
                        diagnostics
                            .add(span, format!("Module not found: {}", path.display()))
                            .add_extra("Importing here", Some(span));
                        return self.error(span);
                    };
                    let Some(root) = asts
                        .parse(module, source)
                        .ok()
                        .and_then(|root| root.root_id())
                    else {
                        diagnostics
                            .add(span, format!("Failed to parse module: {}", path.display()))
                            .add_extra("Importing here", Some(span));
                        return self.error(span);
                    };
                    let savepoint = self.envs.push();
                    let val = self.type_term_inner(asts, root, diagnostics, modules, 0);
                    self.envs.restore(savepoint);
                    val
                }
                [first, condition, then_branch] if Self::is_symbol(asts, first, "if") => {
                    tracing::trace!("Infering if expression");
                    let cond = self.type_term(asts, condition, diagnostics, modules, level);
                    let boolean = self.primitive(Self::BOOLEAN, Self::span_of(condition, asts));
                    self.constrain(cond, boolean, diagnostics);

                    let then = self.type_term(asts, then_branch, diagnostics, modules, level);
                    let else_ = self.unit(span);

                    let merged = self.fresh_var(Self::span_of(then_branch, asts), level);
                    self.constrain(then, merged, diagnostics);
                    self.constrain(else_, merged, diagnostics);

                    merged
                }
                [first, condition, then_branch, else_branch]
                    if Self::is_symbol(asts, first, "if") =>
                {
                    tracing::trace!("Infering if expression");
                    let cond = self.type_term(asts, condition, diagnostics, modules, level);
                    tracing::trace!("Condition type: {}", cond.0);
                    let boolean = self.primitive(Self::BOOLEAN, Self::span_of(condition, asts));
                    self.constrain(cond, boolean, diagnostics);

                    let then = self.type_term(asts, then_branch, diagnostics, modules, level);
                    let else_ = self.type_term(asts, else_branch, diagnostics, modules, level);

                    let merged = self.fresh_var(Self::span_of(then_branch, asts), level);
                    self.constrain(then, merged, diagnostics);
                    self.constrain(else_, merged, diagnostics);

                    merged
                }
                [first, pattern_id, body] if Self::is_symbol(asts, first, "fn") => {
                    tracing::trace!("Infering function expression");
                    let pattern = match Pattern::parse(pattern_id, asts) {
                        Ok(pattern) => pattern,
                        Err(e) => {
                            diagnostics.add_sexp(
                                asts,
                                pattern_id,
                                format!("Unreadable pattern: {e}"),
                            );
                            return self.error(Self::span_of(pattern_id, asts));
                        }
                    };

                    self.envs.push();
                    let pattern = self.type_pattern(
                        asts,
                        pattern,
                        level,
                        TypeSchemeKind::Monomorphic,
                        &mut Default::default(),
                    );
                    let body = self.type_term(asts, body, diagnostics, modules, level);
                    self.envs.pop();

                    self.function(pattern, body, span)
                }
                [first, pattern_id, _captured, body] if Self::is_symbol(asts, first, "cl") => {
                    tracing::trace!("Infering closure expression");

                    // For now lets ignore captured
                    let pattern = match Pattern::parse(pattern_id, asts) {
                        Ok(pattern) => pattern,
                        Err(e) => {
                            diagnostics.add_sexp(
                                asts,
                                pattern_id,
                                format!("Unreadable pattern: {e}"),
                            );
                            return self.error(Self::span_of(pattern_id, asts));
                        }
                    };

                    self.envs.push();
                    let pattern = self.type_pattern(
                        asts,
                        pattern,
                        level,
                        TypeSchemeKind::Monomorphic,
                        &mut Default::default(),
                    );

                    let body = self.type_term(asts, body, diagnostics, modules, level);
                    self.envs.pop();

                    self.function(pattern, body, span)
                }
                [first, last] if Self::is_symbol(asts, first, "do") => {
                    tracing::trace!("Infering do expression");

                    self.envs.push();
                    let body = self.type_term(asts, last, diagnostics, modules, level);
                    self.envs.pop();
                    body
                }
                [first, ref args @ .., last] if Self::is_symbol(asts, first, "do") => {
                    tracing::trace!("Infering do expression");

                    self.envs.push();
                    for arg in args.to_vec() {
                        self.type_term(asts, arg, diagnostics, modules, level);
                    }
                    let last = self.type_term(asts, last, diagnostics, modules, level);
                    self.envs.pop();
                    last
                }
                [first, last] if Self::is_symbol(asts, first, "top-level") => {
                    tracing::trace!("Infering do expression");

                    self.envs.push();
                    self.type_term(asts, last, diagnostics, modules, level)
                }
                [first, ref args @ .., last] if Self::is_symbol(asts, first, "top-level") => {
                    tracing::trace!("Infering do expression");

                    self.envs.push();
                    for arg in args.to_vec() {
                        self.type_term(asts, arg, diagnostics, modules, level);
                    }
                    self.type_term(asts, last, diagnostics, modules, level)
                }
                [first, pattern_id, value] if Self::is_symbol(asts, first, "let") => {
                    tracing::trace!("Infering let expression");
                    let pattern = match Pattern::parse(pattern_id, asts) {
                        Ok(pattern) => pattern,
                        Err(err) => {
                            diagnostics.add_sexp(
                                asts,
                                pattern_id,
                                format!("Unreadable pattern: {err}"),
                            );
                            return self.error(Self::span_of(pattern_id, asts));
                        }
                    };
                    let rhs_ty = self.type_term(asts, value, diagnostics, modules, level + 1);
                    let scheme = if Self::is_worth_generalizing(value, asts) {
                        TypeSchemeKind::Polymorphic { level }
                    } else {
                        TypeSchemeKind::Monomorphic
                    };
                    let bound = self.type_pattern(
                        asts,
                        pattern,
                        level + 1,
                        scheme,
                        &mut Default::default(),
                    );
                    self.constrain(rhs_ty, bound, diagnostics);
                    self.unit(span)
                }
                [first, ref bindings @ ..]
                    if Self::is_symbols(asts, first, &["let-rec", "let*"]) =>
                {
                    let mut bounds = vec![];
                    let (bindings, remainder) = bindings.as_chunks::<2>();
                    if !remainder.is_empty() {
                        let first = remainder[0];
                        diagnostics.add_sexp(asts, first, "let*: found pattern that lacks a value");
                    }
                    let mut stored_keys = Default::default();
                    for &[pattern, value] in bindings {
                        let pattern = match Pattern::parse(pattern, asts) {
                            Ok(pat) => pat,
                            Err(err) => {
                                diagnostics.add_sexp(
                                    asts,
                                    pattern,
                                    format!("Unreadable pattern: {err}"),
                                );
                                continue;
                            }
                        };
                        let bound = self.type_pattern(
                            asts,
                            pattern,
                            level + 1,
                            TypeSchemeKind::Monomorphic,
                            &mut stored_keys,
                        );
                        bounds.push((bound, value));
                    }
                    for (bound, value) in bounds {
                        let value = self.type_term(asts, value, diagnostics, modules, level + 1);
                        if let Some(key) = stored_keys.remove(&bound) {
                            self.envs.set(
                                &key,
                                InferedTypeScheme::Polymorphic(InferedPolymorphicType {
                                    level,
                                    body: bound,
                                }),
                            );
                        }
                        self.constrain(value, bound, diagnostics);
                    }
                    self.unit(span)
                }
                [first, _err] if Self::is_symbol(asts, first, "error") => self.error(span),
                [first, _captured, rest] if Self::is_symbol(asts, first, "thunk") => {
                    tracing::trace!("Infering thunk expression");
                    self.type_term(asts, rest, diagnostics, modules, level)
                }
                [first, value] if Self::is_symbol(asts, first, "ref") => {
                    tracing::trace!("Infering reference expression");
                    let value_type = self.type_term(asts, value, diagnostics, modules, level);
                    // let var = self.fresh_var(span, level);
                    // self.constrain(value_type, var, diagnostics);

                    // self.reference(Some(var), Some(var), span)
                    self.reference(Some(value_type), Some(value_type), span)
                }
                [first, ref args @ ..] if Self::is_symbol(asts, first, "obj/plain") => {
                    tracing::trace!("Infering record expression");
                    let mut fields = IndexMap::new();
                    let args = args.to_vec();
                    let (args, rest) =
                        self.handle_splice(asts, args.into_iter(), diagnostics, modules, level);

                    for ((key_type, key_span), (value, _)) in args.into_iter().tuples() {
                        // let key_type = self.type_term(asts, key, diagnostics, modules, level);
                        let ty = self.get(key_type);
                        tracing::warn!("TY: {ty:?}");
                        let Some(key) =
                            self.find_in_successors(key_type, InferedType::as_keyword_literal)
                        else {
                            diagnostics
                                .add(key_span, "obj/plain: Expected keyword literal")
                                // .add_extra("Record is created here", Some(span))
                                .add_extra("This is not a keyword literal", Some(key_span));
                            return self.error(key_span);
                        };
                        let key = key.to_string();
                        // let value = self.type_term(asts, value, diagnostics, modules, level);
                        fields.insert(key, value);
                    }

                    self.record(fields, rest, span)
                }
                [first, proto, ref args @ ..] if Self::is_symbol(asts, first, "obj/extend") => {
                    tracing::trace!("Infering record extension expression");

                    let mut fields = IndexMap::new();
                    let args = args.to_vec();
                    let proto = self.type_term(asts, proto, diagnostics, modules, level);
                    for (key, value) in args.into_iter().tuples() {
                        let key_type = self.type_term(asts, key, diagnostics, modules, level);
                        let Some(key) =
                            self.find_in_successors(key_type, InferedType::as_keyword_literal)
                        else {
                            diagnostics.add_sexp(asts, key, "obj/extend: Expected keyword");
                            return self.error(Self::span_of(key, asts));
                        };
                        let key = key.to_string();
                        let value = self.type_term(asts, value, diagnostics, modules, level);
                        fields.insert(key, value);
                    }

                    self.record(fields, Some(proto), span)
                }
                [first, ref_mut, value_id] if Self::is_symbol(asts, first, "set") => {
                    tracing::trace!("Infering set reference");
                    let ref_mut = self.type_term(asts, ref_mut, diagnostics, modules, level);
                    let value = self.type_term(asts, value_id, diagnostics, modules, level);
                    let bound = self.reference(Some(value), None, Self::span_of(value_id, asts));
                    self.constrain(ref_mut, bound, diagnostics);
                    value
                }

                [callee, ref args @ ..] => {
                    tracing::trace!("Infering application call");
                    let args = args.to_vec();
                    let callee_type = self.type_term(asts, callee, diagnostics, modules, level);

                    let (arg_types, rest_type) =
                        self.handle_splice(asts, args.iter().copied(), diagnostics, modules, level);

                    let args_span = arg_types
                        .iter()
                        .map(|(_, span)| *span)
                        .reduce(Span::reduce)
                        .unwrap_or(span);

                    let arg_types: Vec<_> = arg_types.into_iter().map(|(arg, _)| arg).collect();

                    let ret_type = self.fresh_var(span, level);

                    let first_arg = arg_types.first().copied();
                    let args = self.tuple(arg_types, rest_type, args_span);
                    let bound = self.applicative(args, ret_type, first_arg, span);

                    self.constrain(callee_type, bound, diagnostics);

                    ret_type
                }
            },
            SExp::Error => self.error(span),
        }
    }

    pub(crate) fn handle_splice(
        &mut self,
        asts: &mut ASTS,
        args: impl Iterator<Item = SExpId>,
        diagnostics: &mut Diagnostics,
        modules: &mut dyn ModuleProvider,
        level: usize,
    ) -> (Vec<(InferedTypeId, Span)>, Option<InferedTypeId>) {
        let mut rest = None;
        let r = &mut rest;

        let args = args
            .flat_map(move |arg| {
                let span = Self::span_of(arg, asts);
                if let Some(list) = Self::as_special_form(asts, arg, "splice") {
                    if let Some(first) = list.get(1) {
                        tracing::trace!("Found splice");
                        let infered = self.type_term(asts, *first, diagnostics, modules, level);
                        *r = Some(infered);
                        return vec![];
                    }
                }
                let ty = self.type_term(asts, arg, diagnostics, modules, level);
                vec![(ty, span)]
            })
            .collect();

        tracing::trace!("Args: {args:?}; Rest: {rest:?}");
        (args, rest)
    }

    pub(crate) fn as_special_form<'a>(
        asts: &'a ASTS,
        list_id: SExpId,
        name: &str,
    ) -> Option<&'a [SExpId]> {
        let list = asts.get(list_id);
        let list = list.as_list()?;
        let first = list.first()?;
        let first = asts.get(*first);
        let first = first.as_symbol()?;
        if first == name { Some(list) } else { None }
    }

    fn is_worth_generalizing(sexp: SExpId, asts: &ASTS) -> bool {
        if !Self::is_expression_value(sexp, asts) {
            return false;
        }
        Self::has_unknown_variable(sexp, asts)
    }

    fn has_unknown_variable(sexp: SExpId, asts: &ASTS) -> bool {
        match &**asts.get(sexp) {
            SExp::Number(_)
            | SExp::String(_)
            | SExp::Bool(_)
            | SExp::Symbol(_)
            | SExp::Keyword(_)
            | SExp::Error => false,
            SExp::List(sexp_ids) => match sexp_ids[..] {
                [] => false,
                [first, ..] if Self::is_symbols(asts, first, &["fn", "cl"]) => true,
                _ => false,
            },
        }
    }

    fn is_expression_value(sexp: SExpId, asts: &ASTS) -> bool {
        match &**asts.get(sexp) {
            SExp::Number(_)
            | SExp::String(_)
            | SExp::Bool(_)
            | SExp::Symbol(_)
            | SExp::Error
            | SExp::Keyword(_) => true,
            SExp::List(sexp_ids) => match sexp_ids.as_slice() {
                [] => true,
                [first, ..] if Self::is_symbols(asts, *first, &["fn", "cl"]) => true,
                [first, rest @ ..] if Self::is_symbols(asts, *first, &["do", "if", "let"]) => rest
                    .iter()
                    .all(|sexp_id| Self::is_expression_value(*sexp_id, asts)),

                _ => false,
            },
        }
    }

    fn type_pattern(
        &mut self,
        asts: &ASTS,
        pattern: Pattern,
        level: usize,
        scheme: TypeSchemeKind,
        stored_keys: &mut HashMap<InferedTypeId, String>,
    ) -> InferedTypeId {
        let (ty, _kind) =
            self.type_pattern_inner(asts, pattern, level, scheme, stored_keys, PartOf::Nothing);
        ty
    }

    fn type_pattern_inner(
        &mut self,
        asts: &ASTS,
        pattern: Pattern,
        level: usize,
        scheme: TypeSchemeKind,
        stored_keys: &mut HashMap<InferedTypeId, String>,
        part_of: PartOf,
    ) -> (InferedTypeId, TypeOrRest) {
        tracing::trace!(
            "Infering pattern: {:?} - {} lvl {:?}",
            pattern,
            level,
            scheme
        );
        match pattern {
            Pattern::Hole(span, id) => {
                let var = self.fresh_var(span, level);
                self.add_sexp(asts, id, var);
                (var, TypeOrRest::Type)
            }
            Pattern::Single(key, span, id) => {
                let var = self.fresh_var(span, level);
                self.add_sexp(asts, id, var);
                let mut bound_var = var;
                // When binding rest pattern there is a disparity between the type in the object
                // and the type bound to a variable.
                // Example:
                // Lets say we have a tuple full of integers, represented in a type by (1, 2, 3).
                // When we create a pattern (:a, ..:rest) we store in the tuple (1)&{rest:3 or 3} instead of
                // (1)&{rest:List<2 or 3>}.
                // However, when binding a rest pattern, we must ensure that the `:rest` variable has a type of list
                // instead of a type of its element.
                match part_of {
                    PartOf::ListRest => {
                        bound_var = self.list(var, span);
                    }
                    PartOf::ObjectRest => todo!(),
                    _ => (),
                }
                let scheme = match scheme {
                    TypeSchemeKind::Monomorphic => InferedTypeScheme::Monomorphic(bound_var),
                    TypeSchemeKind::Polymorphic { level } => {
                        InferedTypeScheme::Polymorphic(InferedPolymorphicType {
                            level,
                            body: bound_var,
                        })
                    }
                };
                self.envs.set(&key, scheme);
                stored_keys.insert(bound_var, key);
                (var, TypeOrRest::Type)
            }
            Pattern::Splice(s, _span, id) => {
                // :
                let (s, _) = self.type_pattern_inner(
                    asts,
                    *s,
                    level,
                    scheme,
                    stored_keys,
                    part_of.into_rest(),
                );
                self.add_sexp(asts, id, s);
                (s, TypeOrRest::Rest)
            }
            Pattern::List(patterns, span, id) => {
                let mut bounds = Vec::new();
                let mut rest = None;
                for pattern in patterns {
                    let (bound, kind) = self.type_pattern_inner(
                        asts,
                        pattern,
                        level,
                        scheme,
                        stored_keys,
                        PartOf::List,
                    );
                    match kind {
                        TypeOrRest::Type => {
                            bounds.push(bound);
                        }
                        TypeOrRest::Rest => {
                            rest = Some(bound);
                        }
                    }
                }
                // let rest = rest.map(|rest| self.list(rest, span));

                let tuple = self.tuple(bounds, rest, span);
                (self.add_sexp(asts, id, tuple), TypeOrRest::Type)
            }
            Pattern::Object(patterns, span, id) => {
                let mut bounds = IndexMap::new();
                for (key, pattern) in patterns {
                    let (bound, _) = self.type_pattern_inner(
                        asts,
                        pattern,
                        level,
                        scheme,
                        stored_keys,
                        PartOf::Object,
                    );
                    bounds.insert(key, bound);
                }

                let record = self.record(bounds, None, span);
                (self.add_sexp(asts, id, record), TypeOrRest::Type)
            }
        }
    }
}
