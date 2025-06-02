use crate::ast::SExpId;

use super::Visitor;

pub struct Quote {
    pub id: SExpId,
    pub quoted: SExpId,
}

impl Quote {
    pub fn visit<'a>(self, visitor: &mut impl Visitor<'a>) -> Option<SExpId> {
        visitor.visit_quote(self)
    }

    pub fn visit_quoted<'a>(&mut self, visitor: &mut impl Visitor<'a>) -> Option<SExpId> {
        let new_id = visitor.visit_sexp(self.quoted)?;
        self.quoted = new_id;
        self.id = visitor.helper_mut().assemble(("quote", new_id));
        Some(self.id)
    }
}
