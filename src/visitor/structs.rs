use crate::ast::SExpId;

use super::Visitor;

pub struct Struct {
    pub id: SExpId,
}

impl Struct {
    pub fn visit_struct_values<'a>(&mut self, visitor: &mut impl Visitor<'a>) {
        let _ = visitor;
    }
}
