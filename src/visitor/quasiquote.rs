use crate::{ast::SExpId, source::Span};

use super::{Visitor, unquote::UnquoteVisitor};

pub struct Quasiquote {
    pub id: SExpId,
    pub quoted: SExpId,
    pub span: Span,
}

impl Quasiquote {
    pub fn visit<'a>(self, visitor: &mut impl Visitor<'a>) -> Option<SExpId> {
        visitor.visit_quasiquote(self)
    }

    pub fn visit_unquote<'a>(&mut self, visitor: &mut impl Visitor<'a>) -> Option<SExpId> {
        let mut visitor = UnquoteVisitor(visitor);
        let new_id = visitor.visit_sexp(self.quoted)?;
        self.quoted = new_id;
        self.id = visitor.helper_mut().assemble(("quasiquote", new_id));
        Some(self.id)
    }
}
