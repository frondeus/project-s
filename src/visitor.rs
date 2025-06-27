use crate::{
    ast::{SExp, SExpId},
    source::Spanned,
};

mod helper;
mod list;
mod quasiquote;
mod quote;
mod unquote;

pub use helper::VisitorHelper;
pub use list::List;
pub use quasiquote::Quasiquote;
pub use quote::Quote;
pub use unquote::Unquote;

pub trait Visitor<'a>: Sized {
    fn helper_mut(&mut self) -> &mut VisitorHelper<'a>;
    fn helper(&self) -> &VisitorHelper<'a>;

    fn visit_sexp(&mut self, id: Spanned<SExpId>) -> Option<Spanned<SExpId>> {
        let sexp = self.helper().asts.get(id.inner());
        match &**sexp {
            SExp::List(list) => {
                let first = list.first().copied()?;
                let first = self.helper().spanned(first);
                if self.helper().is_symbol(first, "quote") {
                    let quoted = self.helper().spanned(list[1]);
                    let quote = Quote {
                        token: first,
                        span: sexp.span,
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
    fn visit_list(&mut self, mut list: List) -> Option<Spanned<SExpId>> {
        list.visit_children(self);
        list.id()
    }
    fn visit_atom(&mut self, id: Spanned<SExpId>) -> Option<Spanned<SExpId>> {
        let _ = id;
        None
    }
    fn visit_quote(&mut self, quote: Quote) -> Option<Spanned<SExpId>> {
        let _ = quote;
        None
    }
    fn visit_quasiquote(&mut self, quasiquote: Quasiquote) -> Option<Spanned<SExpId>> {
        let _ = quasiquote;
        None
    }
    fn visit_unquote(&mut self, mut unquote: Unquote) -> Option<Spanned<SExpId>> {
        let new_id = self.visit_sexp(unquote.unquoted)?;
        unquote.id = self
            .helper_mut()
            .assemble((unquote.token, new_id), unquote.span);
        Some(unquote.id)
    }
}
