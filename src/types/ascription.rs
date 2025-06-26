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
        sexp: SExpId,
        canon: &mut CanonicalBuilder,
        diagnostics: &mut Diagnostics,
    ) -> CanonId {
        let sexp = asts.get(sexp);
        match &sexp.item {
            SExp::Keyword(symbol) if symbol == "number" => canon.add(Canonical::Number),
            SExp::Keyword(symbol) if symbol == "string" => canon.add(Canonical::String),
            SExp::Keyword(symbol) if symbol == "bool" => canon.add(Canonical::Bool),
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
                _ => {
                    diagnostics.add(sexp.span.clone(), format!("Unknown type: {:?}", sexp.item));
                    canon.add(Canonical::Error)
                }
            },
            _ => {
                diagnostics.add(sexp.span.clone(), format!("Unknown type: {:?}", sexp.item));
                canon.add(Canonical::Error)
            }
        }
    }
}
