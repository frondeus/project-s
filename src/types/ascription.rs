use std::collections::{HashMap, hash_map::Entry};

use itertools::Itertools;

use crate::{
    ast::{ASTS, SExp, SExpId},
    diagnostics::{Diagnostics, SExpDiag},
    source::Span,
    types::core::Literal,
};

use super::{
    TypeEnv,
    canonical::{CanonId, Canonical, CanonicalBuilder},
};

impl TypeEnv {
    pub(crate) fn parse_type(
        asts: &ASTS,
        id: SExpId,
        canon: &mut CanonicalBuilder,
        diagnostics: &mut Diagnostics,
    ) -> CanonId {
        let mut vars = HashMap::new();
        Self::parse_type_inner(asts, id, canon, diagnostics, &mut vars)
    }
    pub(crate) fn parse_type_inner(
        asts: &ASTS,
        id: SExpId,
        canon: &mut CanonicalBuilder,
        diagnostics: &mut Diagnostics,
        vars: &mut HashMap<String, CanonId>,
    ) -> CanonId {
        let sexp = asts.get(id);
        let span = Some(sexp.span);
        const PRIMITIVES: &[&str] = &["number", "string", "bool", "keyword"];
        match &**sexp {
            SExp::Bool(b) => canon.add(Canonical::Literal(Literal::Bool(*b), span)),
            SExp::Number(n) => canon.add(Canonical::Literal(Literal::Number(*n), span)),
            SExp::String(s) => canon.add(Canonical::Literal(Literal::String(s.to_string()), span)),
            SExp::Keyword(symbol) if PRIMITIVES.contains(&symbol.as_str()) => {
                canon.add(Canonical::Primitive(symbol.to_string(), span))
            }
            SExp::Keyword(k) => {
                canon.add(Canonical::Literal(Literal::Keyword(k.to_string()), span))
            }
            SExp::Symbol(symbol) if symbol == "_" => canon.add(Canonical::Wildcard(span)),
            SExp::List(items) => match &items[..] {
                &[first, inner] if Self::is_symbol(asts, first, "list") => {
                    let inner = Self::parse_type_inner(asts, inner, canon, diagnostics, vars);
                    canon.add(Canonical::List { item: inner, span })
                }
                &[first, pattern, ret] if Self::is_symbol(asts, first, "fn") => {
                    let pattern = Self::parse_type_inner(asts, pattern, canon, diagnostics, vars);
                    let ret = Self::parse_type_inner(asts, ret, canon, diagnostics, vars);
                    canon.add(Canonical::Func { pattern, ret, span })
                }
                [first, fields_exprs @ ..] if Self::is_symbol(asts, *first, "extend") => {
                    let proto = canon.add(Canonical::Wildcard(span));
                    let mut fields = Vec::new();
                    for (key, value) in fields_exprs.iter().tuples() {
                        let Some(key) = Self::as_keyword(asts, *key) else {
                            diagnostics.add_sexp(
                                asts,
                                *key,
                                format!("Expected keyword, got {key:?}"),
                            );
                            continue;
                        };
                        let value = Self::parse_type_inner(asts, *value, canon, diagnostics, vars);
                        fields.push((key.to_string(), value));
                    }

                    canon.add(Canonical::Record {
                        fields,
                        proto: Some(proto),
                        span,
                    })
                }
                [first, fields_exprs @ ..] if Self::is_symbol(asts, *first, "record") => {
                    let mut fields = Vec::new();
                    for (key, value) in fields_exprs.iter().tuples() {
                        let Some(key) = Self::as_keyword(asts, *key) else {
                            diagnostics.add_sexp(
                                asts,
                                *key,
                                format!("Expected keyword, got {key:?}",),
                            );
                            continue;
                        };
                        let value = Self::parse_type_inner(asts, *value, canon, diagnostics, vars);
                        fields.push((key.to_string(), value));
                    }
                    canon.add(Canonical::Record {
                        fields,
                        proto: None,
                        span,
                    })
                }

                &[first, mut_, inner]
                    if Self::is_symbol(asts, first, "ref")
                        && Self::is_symbol(asts, mut_, "mut") =>
                {
                    let inner = Self::parse_type_inner(asts, inner, canon, diagnostics, vars);
                    canon.add(Canonical::Reference {
                        read: None,
                        write: Some(inner),
                        span,
                    })
                }
                &[first, inner] if Self::is_symbol(asts, first, "ref") => {
                    let inner = Self::parse_type_inner(asts, inner, canon, diagnostics, vars);
                    canon.add(Canonical::Reference {
                        read: Some(inner),
                        write: None,
                        span,
                    })
                }
                &[first, inner] if Self::is_symbol(asts, first, "refmut") => {
                    let inner = Self::parse_type_inner(asts, inner, canon, diagnostics, vars);
                    canon.add(Canonical::Reference {
                        read: Some(inner),
                        write: Some(inner),
                        span,
                    })
                }
                &[first, inner] if Self::is_symbol(asts, first, "quote") => {
                    let inner = asts.get(inner);
                    match inner.as_symbol() {
                        Some(symbol) => {
                            let id = vars.len();
                            let id = vars
                                .entry(symbol.to_string())
                                .or_insert_with(|| canon.add(Canonical::Any(Some(id), span)));
                            *id
                        }
                        _ => {
                            diagnostics
                                .add(inner, format!("Expected symbol, got {}", inner.fmt(asts)));
                            canon.add(Canonical::Error(span))
                        }
                    }
                }
                [first, rest @ ..] if Self::is_symbol(asts, *first, "tuple") => {
                    let mut items = Vec::new();
                    for item in rest {
                        items.push(Self::parse_type_inner(
                            asts,
                            *item,
                            canon,
                            diagnostics,
                            vars,
                        ));
                    }
                    canon.add(Canonical::Tuple { items, span })
                }
                &[first, var_id, value] if Self::is_symbol(asts, first, "as") => {
                    let mut reserved_id = None;

                    let Some(_var) = Self::quote_symbol(
                        asts,
                        var_id,
                        diagnostics,
                        |span, symbol, diagnostics| {
                            reserved_id = Some(vars.len());
                            match vars.entry(symbol.to_string()) {
                                Entry::Occupied(_occupied_entry) => {
                                    diagnostics
                                        .add(span, format!("Symbol {symbol} is already used",));
                                    None
                                }
                                Entry::Vacant(vacant_entry) => {
                                    let entry = vacant_entry
                                        .insert(canon.add(Canonical::Any(reserved_id, Some(span))));
                                    Some(*entry)
                                }
                            }
                        },
                    ) else {
                        return canon.add(Canonical::Error(span));
                    };

                    let value = Self::parse_type_inner(asts, value, canon, diagnostics, vars);

                    canon.add(Canonical::As(reserved_id.unwrap(), value, span))
                }
                rest => {
                    let mut items = Vec::new();
                    for item in rest {
                        items.push(Self::parse_type_inner(
                            asts,
                            *item,
                            canon,
                            diagnostics,
                            vars,
                        ));
                    }
                    canon.add(Canonical::Tuple { items, span })
                }
            },
            _ => {
                diagnostics.add(sexp, format!("Unknown type: {}", asts.fmt(id)));
                canon.add(Canonical::Error(span))
            }
        }
    }

    fn quote_symbol(
        asts: &ASTS,
        id: SExpId,
        diagnostics: &mut Diagnostics,
        with: impl FnOnce(Span, &str, &mut Diagnostics) -> Option<CanonId>,
    ) -> Option<CanonId> {
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
        // let id = vars.len();
        // let id = vars
        //     .entry(symbol.clone())
        //     .or_insert_with(|| canon.add(with(id)));
        // *id
    }
}
