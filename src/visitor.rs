use crate::{
    ast::{SExp, SExpId},
    source::WithSpan,
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

    fn visit_sexp(&mut self, id: SExpId) -> Option<SExpId> {
        let sexp = self.helper().asts.get(id);
        match &sexp.item {
            SExp::List(list) => {
                let first = list.first().copied()?;
                if self.helper().is_symbol(first, "quote") {
                    let quote = Quote {
                        id,
                        quoted: list[1],
                    };
                    return self.visit_quote(quote);
                }
                if self.helper().is_symbol(first, "quasiquote") {
                    let quasiquote = Quasiquote {
                        id,
                        quoted: list[1],
                        span: sexp.span.clone(),
                    };
                    return self.visit_quasiquote(quasiquote);
                }

                let list = List {
                    id,
                    list: list.clone(),
                    edited: false,
                    span: sexp.span.clone(),
                };
                self.visit_list(list)
            }
            _ => self.visit_atom(sexp.clone().map(|_sexp| id)),
        }
    }
    fn visit_list(&mut self, mut list: List) -> Option<SExpId> {
        list.visit_children(self);
        list.id()
    }
    fn visit_atom(&mut self, id: WithSpan<SExpId>) -> Option<SExpId> {
        let _ = id;
        None
    }
    fn visit_quote(&mut self, quote: Quote) -> Option<SExpId> {
        let _ = quote;
        None
    }
    fn visit_quasiquote(&mut self, quasiquote: Quasiquote) -> Option<SExpId> {
        let _ = quasiquote;
        None
    }
    fn visit_unquote(&mut self, mut unquote: Unquote) -> Option<SExpId> {
        let new_id = self.visit_sexp(unquote.unquoted)?;
        unquote.id = self.helper_mut().assemble(("unquote", new_id));
        Some(unquote.id)
    }
}
