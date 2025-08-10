use std::{collections::hash_map::Entry, ops::Deref};

// use crate::modules::ModuleProvider;

use super::*;

const PRIMITIVES: &[&str] = &["number", "string", "bool", "keyword"];

impl TypeEnv {
    fn is_type_value(
        &mut self,
        asts: &mut ASTS,
        id: SExpId,
        diagnostics: &mut Diagnostics,
        level: usize,
    ) -> Result<Option<TypeValue>, InferedTypeId> {
        let sexp = asts.get(id);
        let span = sexp.span;

        Ok(match sexp.deref() {
            SExp::Symbol(s) if s == "_" => Some(TypeValue::Type(self.fresh_var(span, level))),

            SExp::Symbol(s) => {
                let Some(ty) = self.envs.get_type(s) else {
                    diagnostics
                        .add(span, format!("Unknown type: {s}"))
                        .add_extra("Used here", Some(span));
                    return Err(self.error(span));
                };
                Some(ty)
            }
            _ => None,
        })
    }

    pub(crate) fn ascribe(
        &mut self,
        asts: &mut ASTS,
        id: SExpId,
        diagnostics: &mut Diagnostics,
        vars: &mut HashMap<String, InferedTypeId>,
        level: usize,
    ) -> InferedTypeId {
        let sexp = asts.get(id);
        let span = sexp.span;

        match sexp.deref() {
            &SExp::Number(n) => {
                let lit = self.literal(Literal::Number(n), span);
                let num = self.primitive(Self::NUMBER, span);
                self.constrain(lit, num, diagnostics);
                lit
            }
            SExp::String(s) => {
                let lit = self.literal(Literal::String(s.clone()), span);
                let str = self.primitive(Self::STRING, span);
                self.constrain(lit, str, diagnostics);
                lit
            }
            &SExp::Bool(b) => {
                let lit = self.literal(Literal::Bool(b), span);
                let bool = self.primitive(Self::BOOLEAN, span);
                self.constrain(lit, bool, diagnostics);
                lit
            }
            SExp::Symbol(_) => match self.is_type_value(asts, id, diagnostics, level) {
                Err(e) => e,
                Ok(None) => unreachable!(),
                Ok(Some(ty)) => match ty {
                    TypeValue::Type(infered_type_id) => infered_type_id,
                    TypeValue::Constructor { .. } => {
                        diagnostics
                            .add(span, "Used type constructor")
                            .add_extra("Used here", Some(span));
                        self.error(span)
                    }
                },
            },

            SExp::Keyword(symbol) if PRIMITIVES.contains(&symbol.as_str()) => {
                self.primitive(symbol.clone(), span)
            }
            SExp::Keyword(key) => {
                let lit = self.literal(Literal::Keyword(key.clone()), span);
                let kw = self.primitive(Self::KEYWORD, span);
                self.constrain(lit, kw, diagnostics);
                lit
            }
            SExp::List(items) => match items[..] {
                [first, inner] if Self::is_symbol(asts, first, "list") => {
                    let inner = self.ascribe(asts, inner, diagnostics, vars, level);
                    self.list(inner, span)
                }
                [first, pattern, ret] if Self::is_symbol(asts, first, "fn") => {
                    let pattern = self.ascribe(asts, pattern, diagnostics, vars, level);
                    let ret = self.ascribe(asts, ret, diagnostics, vars, level);
                    self.function(pattern, ret, span)
                }
                [first, ref branches @ ..] if Self::is_symbol(asts, first, "enum") => {
                    let (branches, remainder) = branches.as_chunks::<2>();
                    if !remainder.is_empty() {
                        let remainder = remainder[0];
                        diagnostics.add_sexp(asts, remainder, "Unexpected token");
                        return self.error(span);
                    }
                    let mut variants = IndexMap::new();
                    for [tag, fields] in branches.to_vec() {
                        let Some(tag) = Self::as_keyword(asts, tag) else {
                            let tag_span = Self::span_of(tag, asts);
                            diagnostics
                                .add(tag_span, "Expected keyword")
                                .add_extra("Got", Some(tag_span));
                            continue;
                        };
                        let tag = tag.to_string();
                        let fields_id = self.ascribe(asts, fields, diagnostics, vars, level);
                        let ty = self.get(fields_id);
                        match ty {
                            InferedType::Tuple { .. } => {
                                variants.insert(tag, fields_id);
                            }
                            _ => {
                                let tuple = self.tuple(vec![fields_id], None, span);
                                variants.insert(tag, tuple);
                            }
                        }
                    }
                    self.enum_(variants, span)
                }
                [first, ref fields_exprs @ ..]
                    if Self::is_symbols(asts, first, &["record", "obj/plain"]) =>
                {
                    let mut fields = IndexMap::new();

                    let (tuples, remainder) = fields_exprs.as_chunks::<2>();
                    if !remainder.is_empty() {
                        let remainder = remainder[0];
                        diagnostics.add_sexp(asts, remainder, "Unexpected token");
                        return self.error(span);
                    }
                    for [key, value] in tuples.to_vec() {
                        let Some(key) = Self::as_keyword(asts, key) else {
                            let key_span = Self::span_of(key, asts);
                            diagnostics
                                .add(key_span, "Expected keyword")
                                .add_extra("Got", Some(key_span));
                            continue;
                        };
                        let key = key.to_string();
                        let value = self.ascribe(asts, value, diagnostics, vars, level);
                        fields.insert(key, value);
                    }
                    self.record(fields, None, span)
                }
                [first, ref fields_exprs @ ..] if Self::is_symbol(asts, first, "extend") => {
                    let mut fields = IndexMap::new();

                    let (tuples, remainder) = fields_exprs.as_chunks::<2>();
                    if !remainder.is_empty() {
                        let remainder = remainder[0];
                        diagnostics.add_sexp(asts, remainder, "Unexpected token");
                        return self.error(span);
                    }
                    for [key, value] in tuples.to_vec() {
                        let Some(key) = Self::as_keyword(asts, key) else {
                            let key_span = Self::span_of(key, asts);
                            diagnostics
                                .add(key_span, "Expected keyword")
                                .add_extra("Got", Some(key_span));
                            continue;
                        };
                        let key = key.to_string();
                        let value = self.ascribe(asts, value, diagnostics, vars, level);
                        fields.insert(key, value);
                    }
                    let proto = self.fresh_var(span, level);
                    self.record(fields, Some(proto), span)
                }
                [first, inner1, mut_, inner2]
                    if Self::is_symbol(asts, first, "ref")
                        && Self::is_symbol(asts, mut_, "mut") =>
                {
                    let read = self.ascribe(asts, inner1, diagnostics, vars, level);
                    let write = self.ascribe(asts, inner2, diagnostics, vars, level);
                    self.reference(Some(read), Some(write), span)
                }
                [first, inner] if Self::is_symbol(asts, first, "mut") => {
                    let write = self.ascribe(asts, inner, diagnostics, vars, level);
                    self.reference(None, Some(write), span)
                }

                [first, inner] if Self::is_symbol(asts, first, "ref") => {
                    let read = self.ascribe(asts, inner, diagnostics, vars, level);
                    self.reference(Some(read), None, span)
                }
                [first, inner] if Self::is_symbol(asts, first, "refmut") => {
                    let inner = self.ascribe(asts, inner, diagnostics, vars, level);
                    self.reference(Some(inner), Some(inner), span)
                }
                [first, inner] if Self::is_symbol(asts, first, "quote") => {
                    match asts.get(inner).as_symbol() {
                        Some(symbol) => {
                            let id = vars
                                .entry(symbol.to_string())
                                .or_insert_with(|| self.fresh_var(span, level));
                            *id
                        }
                        None => {
                            diagnostics.add_sexp(
                                asts,
                                inner,
                                format!("Expected symbol, got {}", asts.fmt(inner)),
                            );
                            self.error(span)
                        }
                    }
                }
                [first, var_id, value] if Self::is_symbol(asts, first, "as") => {
                    let mut reserved_id = None;
                    let Some(_var) = Self::ascribe_quote(
                        asts,
                        var_id,
                        diagnostics,
                        |span, symbol, diagnostics| {
                            reserved_id = Some(vars.len());
                            match vars.entry(symbol.to_string()) {
                                Entry::Occupied(_occ) => {
                                    diagnostics
                                        .add(span, format!("Symbol {symbol} is already in use"));
                                    None
                                }
                                Entry::Vacant(vacant) => {
                                    let var = self.fresh_var(span, level);
                                    let entry = vacant.insert(var);
                                    Some(*entry)
                                }
                            }
                        },
                    ) else {
                        return self.error(span);
                    };

                    let value = self.ascribe(asts, value, diagnostics, vars, level);
                    self.constrain(value, _var, diagnostics);
                    _var
                }
                [first, ref rest @ ..] if Self::is_symbol(asts, first, "tuple") => {
                    let mut items = Vec::new();
                    for item in rest.to_vec() {
                        let item = self.ascribe(asts, item, diagnostics, vars, level);
                        items.push(item);
                    }
                    self.tuple(items, None, span)
                }
                [first, ref rest @ ..] => {
                    let rest = rest.to_vec();
                    let first_ty = self.is_type_value(asts, first, diagnostics, level);
                    match first_ty {
                        Err(e) => e,
                        Ok(None) => {
                            let first_ty = self.ascribe(asts, first, diagnostics, vars, level);
                            let mut items = vec![first_ty];
                            for item in rest.to_vec() {
                                let item = self.ascribe(asts, item, diagnostics, vars, level);
                                items.push(item);
                            }
                            self.tuple(items, None, span)
                        }
                        Ok(Some(TypeValue::Type(t))) => {
                            let mut items = vec![t];
                            for item in rest.to_vec() {
                                let item = self.ascribe(asts, item, diagnostics, vars, level);
                                items.push(item);
                            }
                            self.tuple(items, None, span)
                        }

                        Ok(Some(TypeValue::Constructor { args, ret })) => {
                            let rest = rest
                                .into_iter()
                                .map(|arg| self.ascribe(asts, arg, diagnostics, vars, level))
                                .collect();
                            self.constructor_call(args, rest, ret)
                        }
                    }
                }
                ref rest => {
                    let mut items = Vec::new();
                    for item in rest.to_vec() {
                        let item = self.ascribe(asts, item, diagnostics, vars, level);
                        items.push(item);
                    }
                    self.tuple(items, None, span)
                }
            },
            SExp::Error => self.error(span),
        }
    }

    pub(crate) fn constructor_call(
        &mut self,
        generics: Vec<InferedTypeId>,
        params: Vec<InferedTypeId>,
        ret: InferedTypeId,
    ) -> InferedTypeId {
        let mut to_replace = HashMap::new();
        for (type_var, arg) in generics.into_iter().zip(params) {
            to_replace.insert(type_var, arg);
        }
        self.replace_vars(&to_replace, ret, &mut false, &mut HashSet::new())
    }

    fn replace_vars(
        &mut self,
        to_replace: &HashMap<InferedTypeId, InferedTypeId>,
        ty: InferedTypeId,
        replaced: &mut bool,
        visited: &mut HashSet<InferedTypeId>,
    ) -> InferedTypeId {
        let new = self.replace_vars_inner(to_replace, ty, visited);
        if new != ty {
            *replaced = true;
        }
        new
    }

    fn replace_vars_inner(
        &mut self,
        to_replace: &HashMap<InferedTypeId, InferedTypeId>,
        ty: InferedTypeId,
        visited: &mut HashSet<InferedTypeId>,
    ) -> InferedTypeId {
        if let Some(replacement) = to_replace.get(&ty) {
            return *replacement;
        }
        if visited.contains(&ty) {
            return ty;
        }
        visited.insert(ty);

        let mut replaced = false;

        let infered = self.get(ty);
        let new = match *infered {
            InferedType::Function { lhs, rhs, span } => {
                let lhs = self.replace_vars(to_replace, lhs, &mut replaced, visited);
                let rhs = self.replace_vars(to_replace, rhs, &mut replaced, visited);

                InferedType::Function { lhs, rhs, span }
            }
            InferedType::Applicative {
                arg,
                ret,
                first_arg,
                span,
            } => {
                let arg = self.replace_vars(to_replace, arg, &mut replaced, visited);
                let ret = self.replace_vars(to_replace, ret, &mut replaced, visited);
                let first_arg = first_arg.map(|first_arg| {
                    self.replace_vars(to_replace, first_arg, &mut replaced, visited)
                });

                InferedType::Applicative {
                    arg,
                    ret,
                    first_arg,
                    span,
                }
            }
            InferedType::Tuple {
                ref items,
                rest,
                span,
            } => {
                let items = items
                    .to_vec()
                    .into_iter()
                    .map(|item| self.replace_vars(to_replace, item, &mut replaced, visited))
                    .collect::<Vec<_>>();

                InferedType::Tuple { items, rest, span }
            }
            InferedType::Record {
                ref fields,
                proto,
                span,
            } => {
                let fields = fields
                    .clone()
                    .into_iter()
                    .map(|(name, ty)| {
                        let ty = self.replace_vars(to_replace, ty, &mut replaced, visited);
                        (name, ty)
                    })
                    .collect();

                InferedType::Record {
                    fields,
                    proto,
                    span,
                }
            }
            InferedType::List { item, span } => {
                let item = self.replace_vars(to_replace, item, &mut replaced, visited);
                InferedType::List { item, span }
            }
            InferedType::Ref { write, read, span } => {
                let write =
                    write.map(|write| self.replace_vars(to_replace, write, &mut replaced, visited));
                let read =
                    read.map(|read| self.replace_vars(to_replace, read, &mut replaced, visited));
                InferedType::Ref { write, read, span }
            }
            InferedType::Module { ref members, span } => {
                let members = members
                    .clone()
                    .into_iter()
                    .map(|(name, ty)| match ty {
                        InferedTypeScheme::Monomorphic(infered_type_id) => {
                            let infered_type_id = self.replace_vars(
                                to_replace,
                                infered_type_id,
                                &mut replaced,
                                visited,
                            );
                            (name, InferedTypeScheme::Monomorphic(infered_type_id))
                        }
                        InferedTypeScheme::Polymorphic(InferedPolymorphicType { level, body }) => {
                            let body = self.replace_vars(to_replace, body, &mut replaced, visited);
                            (
                                name,
                                InferedTypeScheme::Polymorphic(InferedPolymorphicType {
                                    level,
                                    body,
                                }),
                            )
                        }
                    })
                    .collect();

                InferedType::Module { members, span }
            }
            InferedType::Enum { ref variants, span } => {
                let variants = variants
                    .clone()
                    .into_iter()
                    .map(|(name, ty)| {
                        let ty = self.replace_vars(to_replace, ty, &mut replaced, visited);
                        (name, ty)
                    })
                    .collect();

                InferedType::Enum { variants, span }
            }
            _ => return ty,
        };

        if replaced { self.add_infered(new) } else { ty }
    }

    fn ascribe_quote(
        asts: &ASTS,
        id: SExpId,
        diagnostics: &mut Diagnostics,
        with: impl FnOnce(Span, &str, &mut Diagnostics) -> Option<InferedTypeId>,
    ) -> Option<InferedTypeId> {
        let var = asts.get(id);

        let Some(items) = var.as_list() else {
            diagnostics.add(var, format!("Expected quoted symbol, got {}", asts.fmt(id)));
            return None;
        };

        let &[first, inner] = items else {
            diagnostics.add(var, format!("Expected quoted symbol, got {}", asts.fmt(id)));
            return None;
        };

        if !Self::is_symbol(asts, first, "quote") {
            diagnostics.add(var, format!("Expected quoted symbol, got {}", asts.fmt(id)));
            return None;
        }

        let inner = asts.get(inner);

        let Some(symbol) = inner.as_symbol() else {
            diagnostics.add(inner, format!("Expected symbol, got {}", inner.fmt(asts)));
            return None;
        };

        with(inner.span, symbol, diagnostics)
    }
}
