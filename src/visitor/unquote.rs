use crate::ast::{SExp, SExpId};

use super::{List, Quasiquote, Quote, Visitor, VisitorHelper};

pub struct Unquote {
    pub id: SExpId,
    pub unquoted: SExpId,
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
    fn visit_sexp(&mut self, id: SExpId) -> Option<SExpId> {
        let sexp = self.helper().asts.get(id);
        match &**sexp {
            SExp::List(list) => {
                let first = list.first().copied()?;
                if self.helper().is_symbol(first, "unquote") {
                    let unquote = Unquote {
                        id,
                        unquoted: list[1],
                    };
                    return self.0.visit_unquote(unquote);
                }
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
                        span: sexp.span,
                    };
                    return self.visit_quasiquote(quasiquote);
                }

                let list = List {
                    id,
                    list: list.clone(),
                    edited: false,
                    span: sexp.span,
                };
                self.visit_list(list)
            }
            _ => self.visit_atom(sexp.clone().map(|_| id)),
        }
    }
}
