use std::collections::{HashMap, hash_map::Entry};

use itertools::Itertools;

use crate::{
    ast::{ASTS, SExp, SExpId},
    diagnostics::Diagnostics,
    source::Span,
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
        match &sexp.item {
            SExp::Keyword(symbol) if symbol == "number" => canon.add(Canonical::Number),
            SExp::Keyword(symbol) if symbol == "string" => canon.add(Canonical::String),
            SExp::Keyword(symbol) if symbol == "bool" => canon.add(Canonical::Bool),
            SExp::Symbol(symbol) if symbol == "_" => canon.add(Canonical::Skip),
            SExp::List(items) => match &items[..] {
                &[first, inner] if Self::is_symbol(asts, first, "list") => {
                    let inner = Self::parse_type_inner(asts, inner, canon, diagnostics, vars);
                    canon.add(Canonical::List { item: inner })
                }
                &[first, pattern, ret] if Self::is_symbol(asts, first, "fn") => {
                    let pattern = Self::parse_type_inner(asts, pattern, canon, diagnostics, vars);
                    let ret = Self::parse_type_inner(asts, ret, canon, diagnostics, vars);
                    canon.add(Canonical::Func { pattern, ret })
                }
                [first, fields_exprs @ ..] if Self::is_symbol(asts, *first, "record") => {
                    let mut fields = Vec::new();
                    for (key, value) in fields_exprs.iter().tuples() {
                        let Some(key) = Self::as_keyword(asts, *key) else {
                            let span = Self::span_of(*key, asts);
                            diagnostics.add(span, format!("Expected keyword, got {:?}", key));
                            continue;
                        };
                        let value = Self::parse_type_inner(asts, *value, canon, diagnostics, vars);
                        fields.push((key.to_string(), value));
                    }
                    canon.add(Canonical::Struct { fields })
                }

                &[first, mut_, inner]
                    if Self::is_symbol(asts, first, "ref")
                        && Self::is_symbol(asts, mut_, "mut") =>
                {
                    let inner = Self::parse_type_inner(asts, inner, canon, diagnostics, vars);
                    canon.add(Canonical::Reference {
                        read: None,
                        write: Some(inner),
                    })
                }
                &[first, inner] if Self::is_symbol(asts, first, "ref") => {
                    let inner = Self::parse_type_inner(asts, inner, canon, diagnostics, vars);
                    canon.add(Canonical::Reference {
                        read: Some(inner),
                        write: None,
                    })
                }
                &[first, inner] if Self::is_symbol(asts, first, "refmut") => {
                    let inner = Self::parse_type_inner(asts, inner, canon, diagnostics, vars);
                    canon.add(Canonical::Reference {
                        read: Some(inner),
                        write: Some(inner),
                    })
                }
                &[first, inner] if Self::is_symbol(asts, first, "quote") => {
                    let inner = asts.get(inner);
                    match &inner.item {
                        SExp::Symbol(symbol) => {
                            let id = vars.len();
                            let id = vars
                                .entry(symbol.clone())
                                .or_insert_with(|| canon.add(Canonical::Any(Some(id))));
                            *id
                        }
                        _ => {
                            diagnostics.add(
                                inner.span.clone(),
                                format!("Expected symbol, got {:?}", inner.item),
                            );
                            canon.add(Canonical::Error)
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
                    canon.add(Canonical::Tuple { items })
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
                                        .add(span, format!("Symbol {} is already used", symbol));
                                    None
                                }
                                Entry::Vacant(vacant_entry) => {
                                    let entry =
                                        vacant_entry.insert(canon.add(Canonical::Any(reserved_id)));
                                    Some(*entry)
                                }
                            }
                        },
                    ) else {
                        return canon.add(Canonical::Error);
                    };

                    let value = Self::parse_type_inner(asts, value, canon, diagnostics, vars);

                    canon.add(Canonical::As(reserved_id.unwrap(), value))
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
                    canon.add(Canonical::Tuple { items })
                }
            },
            _ => {
                diagnostics.add(sexp.span.clone(), format!("Unknown type: {}", asts.fmt(id)));
                canon.add(Canonical::Error)
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

        let SExp::List(items) = &var.item else {
            diagnostics.add(
                var.span.clone(),
                format!("Expected quoted symbol, got {}", asts.fmt(id)),
            );
            return None;
        };

        let &[first, inner] = &items[..] else {
            diagnostics.add(
                var.span.clone(),
                format!("Expected quoted symbol, got {}", asts.fmt(id)),
            );
            return None;
        };

        if !Self::is_symbol(asts, first, "quote") {
            diagnostics.add(
                var.span.clone(),
                format!("Expected quoted symbol, got {}", asts.fmt(id)),
            );
            return None;
        }

        let inner = asts.get(inner);

        let SExp::Symbol(symbol) = &inner.item else {
            diagnostics.add(
                inner.span.clone(),
                format!("Expected symbol, got {}", inner.fmt(asts)),
            );
            return None;
        };

        with(inner.span.clone(), symbol, diagnostics)
        // let id = vars.len();
        // let id = vars
        //     .entry(symbol.clone())
        //     .or_insert_with(|| canon.add(with(id)));
        // *id
    }
}
