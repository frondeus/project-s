use crate::{
    ast::{AST, ASTS, SExp, SExpId},
    builder::ASTBuilder,
    source::{Span, Spanned},
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

    pub fn get_sexp(&self, id: impl SpannedSExpId) -> &Spanned<SExp> {
        self.asts.get(id.into(self.asts).inner())
    }

    pub fn spanned(&self, id: SExpId) -> Spanned<SExpId> {
        let span = self.asts.get(id).span;
        Spanned::new(id, span)
    }

    pub fn assemble(&mut self, builder: impl ASTBuilder, span: Span) -> Spanned<SExpId> {
        Spanned::new(builder.dep(self.new_ast(), span), span)
    }

    pub fn then_assemble(
        &mut self,
        builder: impl ASTBuilder,
        span: Span,
    ) -> Option<Spanned<SExpId>> {
        Some(self.assemble(builder, span))
    }

    pub fn as_symbol(&self, id: impl SpannedSExpId, name: &str) -> Option<()> {
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

    pub fn is_one_of_special_forms(&self, ids: &[Spanned<SExpId>], one_of: &[&str]) -> bool {
        let Some(first) = ids.first().copied() else {
            return false;
        };
        self.is_one_of(first, one_of)
    }

    pub fn maybe_get_symbol(&self, maybe_id: Option<impl SpannedSExpId>) -> Option<&str> {
        let id = maybe_id?;
        self.get_symbol(id)
    }

    pub fn get_symbol(&self, sexp_id: impl SpannedSExpId) -> Option<&str> {
        self.get_sexp(sexp_id).as_symbol()
    }

    pub fn is_symbol(&self, sexp_id: impl SpannedSExpId, symbol: &str) -> bool {
        self.get_sexp(sexp_id).as_symbol() == Some(symbol)
    }

    pub fn is_one_of(&self, sexp_id: impl SpannedSExpId, symbols: &[&str]) -> bool {
        self.get_sexp(sexp_id)
            .as_symbol()
            .is_some_and(|s| symbols.contains(&s))
    }
}

pub trait SpannedSExpId {
    fn into(self, asts: &ASTS) -> Spanned<SExpId>;
}

impl SpannedSExpId for SExpId {
    fn into(self, asts: &ASTS) -> Spanned<SExpId> {
        let span = asts.get(self).span;
        Spanned::new(self, span)
    }
}

impl SpannedSExpId for Spanned<SExpId> {
    fn into(self, _asts: &ASTS) -> Spanned<SExpId> {
        self
    }
}
