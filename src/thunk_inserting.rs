#![allow(dead_code)]

use crate::{
    ast::{ASTS, SExpId},
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

    fn visit_list(&mut self, mut list: List) -> Option<SExpId> {
        println!("Visiting list: {:?}", self.helper.asts.fmt(list.id));

        if self.helper.is_special_form(&list, "thunk") {
            if let [_key, _captured, body_id] = &list.list[..] {
                let body = self.helper.get_sexp(*body_id);
                if let Some(body) = body.as_list() {
                    let mut list = List {
                        id: *body_id,
                        list: body.to_vec(),
                        edited: false,
                    };
                    if self.helper.is_special_form(&list, "struct") {
                        list.visit_children(self);
                        return list.id();
                    }
                }
            }
        }

        list.visit_children(self);

        if self.helper.is_special_form(&list, "struct") {
            let mut visitor = StructVisitor {
                helper: &mut self.helper,
                using_super: false,
            };
            list.visit_children(&mut visitor);
            if visitor.using_super && !list.edited {
                return self.helper.then_assemble(("thunk", (), list.id));
            }
            return list.id();
        }

        list.id()
    }
}

struct StructVisitor<'a, 'b> {
    helper: &'b mut VisitorHelper<'a>,
    using_super: bool,
}

impl<'a> Visitor<'a> for StructVisitor<'a, '_> {
    fn helper_mut(&mut self) -> &mut VisitorHelper<'a> {
        self.helper
    }
    fn helper(&self) -> &VisitorHelper<'a> {
        self.helper
    }

    fn visit_atom(&mut self, id: SExpId) -> Option<SExpId> {
        if self.helper.is_symbol(id, "super") {
            println!("Found super!");
            self.using_super = true;
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

    #[test]
    fn integration_runtime() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "json-thunk", |input, _deps| {
            eprintln!("---");
            let mut asts = ASTS::new();
            let ast = asts.parse(input).unwrap();
            let root_id = ast.root_id().unwrap();
            let root_id = crate::process_ast(&mut asts, root_id);
            let root_id = ThunkPass::pass(&mut asts, root_id);
            let mut runtime = crate::runtime::Runtime::new(asts);
            runtime.with_prelude();
            let result = runtime.eval(root_id);
            let value = runtime.to_json(result);
            serde_json::to_string_pretty(&value).unwrap()
        })
    }
}
