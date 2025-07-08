use super::*;

impl TypeEnv {
    pub fn type_term(
        &mut self,
        asts: &mut ASTS,
        id: SExpId,
        diagnostics: &mut Diagnostics,
        level: usize,
    ) -> InferedTypeId {
        let ty = self.type_term_inner(asts, id, diagnostics, level);
        let infered = self.get(ty);
        tracing::trace!("Infered ty {} : {}", ty.0, infered);
        self.sexps.insert(id, ty);
        ty
    }

    fn type_term_inner(
        &mut self,
        asts: &mut ASTS,
        id: SExpId,
        diagnostics: &mut Diagnostics,
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
                Some(ty) => ty.instantiate(self, level),
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
                    todo!(": {ty:?} {value:?}")
                }
                [first] if Self::is_symbol(asts, first, "module") => todo!("module"),
                [first, path_id] if Self::is_symbol(asts, first, "import") => {
                    todo!("import {path_id:?}")
                }
                [first, condition, then_branch] if Self::is_symbol(asts, first, "if") => {
                    tracing::trace!("Infering if expression");
                    let cond = self.type_term(asts, condition, diagnostics, level);
                    let boolean = self.primitive(Self::BOOLEAN, Self::span_of(condition, asts));
                    self.constrain(cond, boolean, diagnostics);

                    let then = self.type_term(asts, then_branch, diagnostics, level);
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
                    let cond = self.type_term(asts, condition, diagnostics, level);
                    tracing::trace!("Condition type: {}", cond.0);
                    let boolean = self.primitive(Self::BOOLEAN, Self::span_of(condition, asts));
                    self.constrain(cond, boolean, diagnostics);

                    let then = self.type_term(asts, then_branch, diagnostics, level);
                    let else_ = self.type_term(asts, else_branch, diagnostics, level);

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
                    let pattern = self.type_pattern(pattern, level, TypeSchemeKind::Monomorphic);
                    let body = self.type_term(asts, body, diagnostics, level);
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
                    let pattern = self.type_pattern(pattern, level, TypeSchemeKind::Monomorphic);

                    let body = self.type_term(asts, body, diagnostics, level);
                    self.envs.pop();

                    self.function(pattern, body, span)
                }
                [first, last] if Self::is_symbol(asts, first, "do") => {
                    tracing::trace!("Infering do expression");

                    self.envs.push();
                    let body = self.type_term(asts, last, diagnostics, level);
                    self.envs.pop();
                    body
                }
                [first, ref args @ .., last] if Self::is_symbol(asts, first, "do") => {
                    tracing::trace!("Infering do expression");

                    self.envs.push();
                    for arg in args.to_vec() {
                        self.type_term(asts, arg, diagnostics, level);
                    }
                    let last = self.type_term(asts, last, diagnostics, level);
                    self.envs.pop();
                    last
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
                    let rhs_ty = self.type_term(asts, value, diagnostics, level + 1);
                    let scheme = if Self::is_expression_value(value, asts) {
                        TypeSchemeKind::Polymorphic
                    } else {
                        TypeSchemeKind::Monomorphic
                    };
                    let bound = self.type_pattern(pattern, level, scheme);
                    self.constrain(rhs_ty, bound, diagnostics);
                    self.unit(span)
                }
                [first, ref bindings @ ..]
                    if Self::is_symbols(asts, first, &["let-rec", "let*"]) =>
                {
                    let mut patterns = vec![];
                    let (bindings, remainder) = bindings.as_chunks::<2>();
                    if !remainder.is_empty() {
                        let first = remainder[0];
                        diagnostics.add_sexp(asts, first, "let*: found pattern that lacks a value");
                    }
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
                        self.type_pattern(pattern.clone(), level + 1, TypeSchemeKind::Monomorphic);
                        patterns.push((pattern, value));
                    }
                    for (pattern, value) in patterns {
                        let value = self.type_term(asts, value, diagnostics, level + 1);
                        let bound = self.type_pattern(pattern, level, TypeSchemeKind::Polymorphic);
                        self.constrain(value, bound, diagnostics);
                    }
                    self.unit(span)
                }
                [first, _err] if Self::is_symbol(asts, first, "error") => self.error(span),
                [first, _captured, rest] if Self::is_symbol(asts, first, "thunk") => {
                    tracing::trace!("Infering thunk expression");
                    self.type_term(asts, rest, diagnostics, level)
                }
                [first, value] if Self::is_symbol(asts, first, "ref") => {
                    tracing::trace!("Infering reference expression");
                    let value_type = self.type_term(asts, value, diagnostics, level);
                    // let var = self.fresh_var(span, level);
                    // self.constrain(value_type, var, diagnostics);

                    // self.reference(Some(var), Some(var), span)
                    self.reference(Some(value_type), Some(value_type), span)
                }
                [first, ref args @ ..] if Self::is_symbol(asts, first, "obj/plain") => {
                    tracing::trace!("Infering record expression");
                    let mut fields = Vec::new();
                    for (key, value) in args.to_vec().into_iter().tuples() {
                        let Some(key) = Self::as_keyword(asts, key) else {
                            diagnostics.add_sexp(asts, key, "Expected keyword");
                            return self.error(Self::span_of(key, asts));
                        };
                        let key = key.to_string();
                        let value = self.type_term(asts, value, diagnostics, level);
                        fields.push((key, value));
                    }

                    self.record(fields, None, span)
                }
                [first, proto, ref args @ ..] if Self::is_symbol(asts, first, "obj/extend") => {
                    tracing::trace!("Infering record extension expression");

                    let mut fields = Vec::new();
                    let args = args.to_vec();
                    let proto = self.type_term(asts, proto, diagnostics, level);
                    for (key, value) in args.into_iter().tuples() {
                        let Some(key) = Self::as_keyword(asts, key) else {
                            diagnostics.add_sexp(asts, key, "Expected keyword");
                            return self.error(Self::span_of(key, asts));
                        };
                        let key = key.to_string();
                        let value = self.type_term(asts, value, diagnostics, level);
                        fields.push((key, value));
                    }

                    self.record(fields, Some(proto), span)
                }
                [first, ref_mut, value_id] if Self::is_symbol(asts, first, "set") => {
                    tracing::trace!("Infering set reference");
                    let ref_mut = self.type_term(asts, ref_mut, diagnostics, level);
                    let value = self.type_term(asts, value_id, diagnostics, level);
                    let bound = self.reference(Some(value), None, Self::span_of(value_id, asts));
                    self.constrain(ref_mut, bound, diagnostics);
                    value
                }

                [callee, ref args @ ..] => {
                    tracing::trace!("Infering application call");
                    let args = args.to_vec();
                    let callee_type = self.type_term(asts, callee, diagnostics, level);

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

                    let arg_types = args
                        .iter()
                        .map(|arg| self.type_term(asts, *arg, diagnostics, level))
                        .collect::<Vec<_>>();

                    let ret_type = self.fresh_var(span, level);

                    let first_arg = arg_types.first().copied();
                    let args = self.tuple(arg_types, args_span);
                    let bound = self.applicative(args, ret_type, first_arg, span);

                    self.constrain(callee_type, bound, diagnostics);

                    ret_type
                }
            },
            SExp::Error => self.error(span),
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
        pattern: Pattern,
        level: usize,
        scheme: TypeSchemeKind,
    ) -> InferedTypeId {
        tracing::trace!(
            "Infering pattern: {:?} - {} lvl {:?}",
            pattern,
            level,
            scheme
        );
        match pattern {
            Pattern::Single(key, span, id) => {
                let var = self.fresh_var(span, level);
                self.add_sexp(id, var);
                // self.envs.set(&key, core::Scheme::Monomorphic(value));
                let scheme = match scheme {
                    TypeSchemeKind::Monomorphic => TypeScheme::Monomorphic(var),
                    TypeSchemeKind::Polymorphic => {
                        TypeScheme::Polymorphic(PolymorphicType { level, body: var })
                    }
                };
                self.envs.set(&key, scheme);
                var
            }
            Pattern::List(patterns, span, id) => {
                let mut bounds = Vec::new();
                for pattern in patterns {
                    let bound = self.type_pattern(pattern, level, scheme);
                    bounds.push(bound);
                }

                let tuple = self.tuple(bounds, span);
                self.add_sexp(id, tuple)
            }
            Pattern::Object(patterns, span, id) => {
                let mut bounds = Vec::new();
                for (key, pattern) in patterns {
                    let bound = self.type_pattern(pattern, level, scheme);
                    bounds.push((key, bound));
                }

                let record = self.record(bounds, None, span);
                self.add_sexp(id, record)
            }
        }
    }
}
