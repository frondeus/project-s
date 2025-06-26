use itertools::Itertools;

use crate::{
    ast::{ASTS, SExp, SExpId},
    diagnostics::Diagnostics,
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
        let sexp = asts.get(id);
        match &sexp.item {
            SExp::Keyword(symbol) if symbol == "number" => canon.add(Canonical::Number),
            SExp::Keyword(symbol) if symbol == "string" => canon.add(Canonical::String),
            SExp::Keyword(symbol) if symbol == "bool" => canon.add(Canonical::Bool),
            SExp::Symbol(symbol) if symbol == "_" => canon.add(Canonical::Skip),
            SExp::List(items) => match &items[..] {
                &[first, inner] if Self::is_symbol(asts, first, "list") => {
                    let inner = Self::parse_type(asts, inner, canon, diagnostics);
                    canon.add(Canonical::List { item: inner })
                }
                [first, rest @ ..] if Self::is_symbol(asts, *first, "tuple") => {
                    let mut items = Vec::new();
                    for item in rest {
                        items.push(Self::parse_type(asts, *item, canon, diagnostics));
                    }
                    canon.add(Canonical::Tuple { items })
                }
                &[first, pattern, ret] if Self::is_symbol(asts, first, "fn") => {
                    let pattern = Self::parse_type(asts, pattern, canon, diagnostics);
                    let ret = Self::parse_type(asts, ret, canon, diagnostics);
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
                        let value = Self::parse_type(asts, *value, canon, diagnostics);
                        fields.push((key.to_string(), value));
                    }
                    canon.add(Canonical::Struct { fields })
                }

                &[first, mut_, inner]
                    if Self::is_symbol(asts, first, "ref")
                        && Self::is_symbol(asts, mut_, "mut") =>
                {
                    let inner = Self::parse_type(asts, inner, canon, diagnostics);
                    canon.add(Canonical::Reference {
                        read: None,
                        write: Some(inner),
                    })
                }
                &[first, inner] if Self::is_symbol(asts, first, "ref") => {
                    let inner = Self::parse_type(asts, inner, canon, diagnostics);
                    canon.add(Canonical::Reference {
                        read: Some(inner),
                        write: None,
                    })
                }
                &[first, inner] if Self::is_symbol(asts, first, "refmut") => {
                    let inner = Self::parse_type(asts, inner, canon, diagnostics);
                    canon.add(Canonical::Reference {
                        read: Some(inner),
                        write: Some(inner),
                    })
                }
                _ => {
                    diagnostics.add(sexp.span.clone(), format!("Unknown type: {}", asts.fmt(id)));
                    canon.add(Canonical::Error)
                }
            },
            _ => {
                diagnostics.add(sexp.span.clone(), format!("Unknown type: {}", asts.fmt(id)));
                canon.add(Canonical::Error)
            }
        }
    }
}
