use crate::{
    ast::SExpId,
    source::{Span, Spanned, WithSpan},
};

use super::Visitor;

pub struct List {
    pub id: Spanned<SExpId>,
    pub list: Vec<Spanned<SExpId>>,
    pub edited: bool,
    pub span: Span,
}

impl WithSpan for List {
    fn span(&self) -> Span {
        self.span
    }
}

impl List {
    pub fn visit<'a>(self, visitor: &mut impl Visitor<'a>) -> Option<Spanned<SExpId>> {
        visitor.visit_list(self)
    }

    pub fn id(self) -> Option<Spanned<SExpId>> {
        if self.edited { Some(self.id) } else { None }
    }

    pub fn visit_children<'a>(&mut self, visitor: &mut impl Visitor<'a>) {
        for id in &mut self.list {
            if let Some(new_id) = visitor.visit_sexp(*id) {
                *id = new_id;
                self.edited = true;
            }
        }
        if self.edited {
            self.id = visitor
                .helper_mut()
                .assemble(self.list.as_slice(), self.span);
        }
    }
}
