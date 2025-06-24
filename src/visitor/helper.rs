use crate::{
    ast::{AST, ASTS, SExp, SExpId},
    builder::ASTBuilder,
    source::WithSpan,
};

use super::List;

pub struct VisitorHelper<'a> {
    pub asts: &'a mut ASTS,
    new_ast_id: usize,
}

impl<'a> VisitorHelper<'a> {
    pub fn new(asts: &'a mut ASTS) -> Self {
        let new_ast = asts.new_ast();
        let ast_id = asts.add_ast(new_ast);
        Self {
            asts,
            new_ast_id: ast_id,
        }
    }
}

impl VisitorHelper<'_> {
    fn new_ast(&mut self) -> &mut AST {
        self.asts.get_ast_by_generation(self.new_ast_id)
    }

    pub fn get_sexp(&self, id: SExpId) -> &WithSpan<SExp> {
        self.asts.get(id)
    }

    pub fn assemble(&mut self, builder: impl ASTBuilder) -> SExpId {
        builder.assemble(self.new_ast())
    }

    pub fn then_assemble(&mut self, builder: impl ASTBuilder) -> Option<SExpId> {
        Some(self.assemble(builder))
    }

    pub fn as_symbol(&self, id: SExpId, name: &str) -> Option<()> {
        if self.is_symbol(id, name) {
            Some(())
        } else {
            None
        }
    }

    pub fn is_special_form(&self, list: &List, name: &str) -> bool {
        let Some(first) = list.list.first().copied() else {
            return false;
        };
        self.is_symbol(first, name)
    }

    pub fn is_one_of_special_forms(&self, ids: &[SExpId], one_of: &[&str]) -> bool {
        let Some(first) = ids.first().copied() else {
            return false;
        };
        self.is_one_of(first, one_of)
    }

    pub fn maybe_get_symbol(&self, maybe_id: Option<SExpId>) -> Option<&str> {
        let id = maybe_id?;
        self.get_symbol(id)
    }

    pub fn get_symbol(&self, sexp_id: SExpId) -> Option<&str> {
        match &self.asts.get(sexp_id).item {
            SExp::Symbol(s) => Some(s),
            _ => None,
        }
    }

    pub fn is_symbol(&self, sexp_id: SExpId, symbol: &str) -> bool {
        match &self.asts.get(sexp_id).item {
            SExp::Symbol(s) => s == symbol,
            _ => false,
        }
    }

    pub fn is_one_of(&self, sexp_id: SExpId, symbols: &[&str]) -> bool {
        match &self.asts.get(sexp_id).item {
            SExp::Symbol(s) => symbols.contains(&s.as_str()),
            _ => false,
        }
    }
}
