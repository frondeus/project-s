#![allow(dead_code)]

use crate::{
    ast::{ASTS, SExp, SExpId},
    visitor::{List, Quasiquote, Visitor, VisitorHelper},
};

pub struct ThunkPass {}

struct StructFinder<'a> {
    helper: VisitorHelper<'a>,
}

impl ThunkPass {
    pub fn pass(asts: &mut ASTS, root: SExpId) -> SExpId {
        let helper = VisitorHelper::new(asts);
        let mut pass = Self {};
        pass.pass_inner(helper, root).unwrap_or(root)
    }

    fn pass_inner(&mut self, helper: VisitorHelper, id: SExpId) -> Option<SExpId> {
        let mut visitor = StructFinder { helper };
        visitor.visit_sexp(id)
    }
}

impl<'a> Visitor<'a> for StructFinder<'a> {
    fn helper_mut(&mut self) -> &mut VisitorHelper<'a> {
        &mut self.helper
    }
    fn helper(&self) -> &VisitorHelper<'a> {
        &self.helper
    }

    fn visit_quasiquote(&mut self, mut quasiquote: Quasiquote) -> Option<SExpId> {
        quasiquote.visit_unquote(self)
    }

    fn visit_atom(&mut self, id: SExpId) -> Option<SExpId> {
        self.helper.as_symbol(id, "+")?;
        self.helper.then_assemble("plus")
    }

    fn visit_list(&mut self, mut list: List) -> Option<SExpId> {
        println!("Visiting list: {:?}", self.helper.asts.fmt(list.id));

        list.visit_children(self);

        if self.helper.is_special_form(&list, "struct") {
            let mut visitor = StructVisitor {
                helper: &mut self.helper,
            };
            list.visit_children(&mut visitor);
            return list.id();
        }

        list.id()
    }
}

struct StructVisitor<'a, 'b> {
    helper: &'b mut VisitorHelper<'a>,
}

impl<'a> Visitor<'a> for StructVisitor<'a, '_> {
    fn helper_mut(&mut self) -> &mut VisitorHelper<'a> {
        self.helper
    }
    fn helper(&self) -> &VisitorHelper<'a> {
        self.helper
    }

    fn visit_quote(&mut self, mut quote: crate::visitor::Quote) -> Option<SExpId> {
        quote.visit_quoted(self)
    }

    fn visit_list(&mut self, list: List) -> Option<SExpId> {
        println!("Visiting list: {:?}", self.helper.asts.fmt(list.id));

        let mut items = list.list.iter();
        while let Some(id) = items.next() {
            match self.helper.get_sexp(*id) {
                SExp::Symbol(_) => {
                    // Key value pair
                    let _value = items.next()?;
                }
                _ => todo!(),
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "thunk", |input, _deps| {
            eprintln!("---");
            let mut asts = ASTS::new();
            let ast = asts.parse(input).unwrap();
            let root_id = ast.root_id().unwrap();
            let new_root = ThunkPass::pass(&mut asts, root_id);
            let output = asts.fmt(new_root);
            output.to_string()
        })
    }
}
