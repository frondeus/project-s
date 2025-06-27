use crate::{
    ast::{SExp, SExpId},
    source::{Span, Spanned},
};

use super::{List, Quasiquote, Quote, Visitor, VisitorHelper};

pub struct Unquote {
    pub token: Spanned<SExpId>,
    pub id: Spanned<SExpId>,
    pub unquoted: Spanned<SExpId>,
    pub span: Span,
}

pub struct UnquoteVisitor<'a, V>(pub &'a mut V);

impl<'a, V> Visitor<'a> for UnquoteVisitor<'_, V>
where
    V: Visitor<'a>,
{
    fn helper_mut(&mut self) -> &mut VisitorHelper<'a> {
        self.0.helper_mut()
    }
    fn helper(&self) -> &VisitorHelper<'a> {
        self.0.helper()
    }
    fn visit_sexp(&mut self, id: Spanned<SExpId>) -> Option<Spanned<SExpId>> {
        let sexp = self.helper().get_sexp(id);
        match &**sexp {
            SExp::List(list) => {
                let first = list.first().copied()?;
                let first = self.helper().spanned(first);
                if self.helper().is_symbol(first, "unquote") {
                    let unquoted = self.helper().spanned(list[1]);
                    let unquote = Unquote {
                        span: sexp.span,
                        token: first,
                        id,
                        unquoted,
                    };
                    return self.0.visit_unquote(unquote);
                }
                if self.helper().is_symbol(first, "quote") {
                    let quoted = self.helper().spanned(list[1]);
                    let quote = Quote {
                        span: sexp.span,
                        token: first,
                        id,
                        quoted,
                    };
                    return self.visit_quote(quote);
                }
                if self.helper().is_symbol(first, "quasiquote") {
                    let quoted = self.helper().spanned(list[1]);
                    let quasiquote = Quasiquote {
                        token: first,
                        id,
                        quoted,
                        span: sexp.span,
                    };
                    return self.visit_quasiquote(quasiquote);
                }

                let list = list
                    .iter()
                    .map(|id| {
                        let span = self.helper().spanned(*id).span;
                        Spanned::new(*id, span)
                    })
                    .collect();

                let list = List {
                    id,
                    list,
                    edited: false,
                    span: sexp.span,
                };
                self.visit_list(list)
            }
            _ => self.visit_atom(id),
        }
    }
}
