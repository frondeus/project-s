use crate::{
    ast::SExpId,
    source::{Span, Spanned},
};

use super::{Visitor, unquote::UnquoteVisitor};

pub struct Quasiquote {
    pub token: Spanned<SExpId>,
    pub id: Spanned<SExpId>,
    pub quoted: Spanned<SExpId>,
    pub span: Span,
}

impl Quasiquote {
    pub fn visit<'a>(self, visitor: &mut impl Visitor<'a>) -> Option<Spanned<SExpId>> {
        visitor.visit_quasiquote(self)
    }

    pub fn visit_unquote<'a>(&mut self, visitor: &mut impl Visitor<'a>) -> Option<Spanned<SExpId>> {
        let mut visitor = UnquoteVisitor(visitor);
        let new_id = visitor.visit_sexp(self.quoted)?;
        self.quoted = new_id;
        self.id = visitor
            .helper_mut()
            .assemble((self.token, new_id), self.span);
        Some(self.id)
    }
}
