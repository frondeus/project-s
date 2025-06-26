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
        &mut self,
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
            _ => {
                diagnostics.add(sexp.span.clone(), format!("Unknown type: {:?}", sexp.item));
                canon.add(Canonical::Error)
            }
        }
    }
}
