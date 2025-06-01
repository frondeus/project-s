#![allow(dead_code)]

use crate::ast::{AST, ASTS, SExp, SExpId};

pub struct PassHelper<'a> {
    pub asts: &'a mut ASTS,
    new_ast_id: usize,
}

impl<'a> PassHelper<'a> {
    fn new(asts: &'a mut ASTS) -> Self {
        let new_ast = asts.new_ast();
        let ast_id = asts.add_ast(new_ast);
        Self {
            asts,
            new_ast_id: ast_id,
        }
    }
}

struct List {
    pub id: SExpId,
    pub list: Vec<SExpId>,
}
impl List {
    pub fn visit(self, helper: &mut PassHelper, visitor: &mut impl Visitor) -> Option<SExpId> {
        visitor.visit_list(helper, self)
    }
    pub fn visit_children(&mut self, helper: &mut PassHelper, visitor: &mut impl Visitor) -> bool {
        let mut edited = false;
        for id in &mut self.list {
            if let Some(new_id) = visitor.visit_sexp(helper, *id) {
                *id = new_id;
                edited = true;
            }
        }
        if edited {
            self.id = helper.new_ast().add_node(SExp::List(self.list.clone()));
        }
        edited
    }
}
struct Quote {
    pub id: SExpId,
    pub quoted: SExpId,
}
struct Quasiquote {
    pub id: SExpId,
    pub quoted: SExpId,
}

impl Quasiquote {
    pub fn visit(self, helper: &mut PassHelper, visitor: &mut impl Visitor) -> Option<SExpId> {
        visitor.visit_quasiquote(helper, self)
    }

    pub fn visit_unquote(
        self,
        helper: &mut PassHelper,
        visitor: &mut impl Visitor,
    ) -> Option<SExpId> {
        let mut visitor = UnquoteVisitor(visitor);
        visitor.visit_sexp(helper, self.quoted)
    }
}

struct Unquote {
    pub id: SExpId,
    pub unquoted: SExpId,
}

struct UnquoteVisitor<'a, V>(&'a mut V);

impl<'a, V> Visitor for UnquoteVisitor<'a, V>
where
    V: Visitor + 'a,
{
    fn visit_sexp(&mut self, helper: &mut PassHelper, id: SExpId) -> Option<SExpId> {
        match helper.asts.get(id) {
            SExp::List(list) => {
                let first = list.first().copied()?;
                if helper.is_one_of(first, &["unquote"]) {
                    let unquote = Unquote {
                        id,
                        unquoted: list[1],
                    };
                    return self.0.visit_unquote(helper, unquote);
                }
                if helper.is_one_of(first, &["quote"]) {
                    let quote = Quote {
                        id,
                        quoted: list[1],
                    };
                    return self.visit_quote(helper, quote);
                }
                if helper.is_one_of(first, &["quasiquote"]) {
                    let quasiquote = Quasiquote {
                        id,
                        quoted: list[1],
                    };
                    return self.visit_quasiquote(helper, quasiquote);
                }

                let list = List {
                    id,
                    list: list.clone(),
                };
                self.visit_list(helper, list)
            }
            _ => self.visit_atom(helper, id),
        }
    }
}

trait Visitor: Sized {
    fn visit_sexp(&mut self, helper: &mut PassHelper, id: SExpId) -> Option<SExpId> {
        match helper.asts.get(id) {
            SExp::List(list) => {
                let first = list.first().copied()?;
                if helper.is_one_of(first, &["quote"]) {
                    let quote = Quote {
                        id,
                        quoted: list[1],
                    };
                    return self.visit_quote(helper, quote);
                }
                if helper.is_one_of(first, &["quasiquote"]) {
                    let quasiquote = Quasiquote {
                        id,
                        quoted: list[1],
                    };
                    return self.visit_quasiquote(helper, quasiquote);
                }

                let list = List {
                    id,
                    list: list.clone(),
                };
                self.visit_list(helper, list)
            }
            _ => self.visit_atom(helper, id),
        }
    }
    fn visit_list(&mut self, helper: &mut PassHelper, mut list: List) -> Option<SExpId> {
        if list.visit_children(helper, self) {
            Some(list.id)
        } else {
            None
        }
    }
    fn visit_atom(&mut self, helper: &mut PassHelper, id: SExpId) -> Option<SExpId> {
        let _ = (helper, id);
        None
    }
    fn visit_quote(&mut self, helper: &mut PassHelper, quote: Quote) -> Option<SExpId> {
        let _ = (helper, quote);
        None
    }
    fn visit_quasiquote(
        &mut self,
        helper: &mut PassHelper,
        quasiquote: Quasiquote,
    ) -> Option<SExpId> {
        let _ = (helper, quasiquote);
        None
    }
    fn visit_unquote(&mut self, helper: &mut PassHelper, unquote: Unquote) -> Option<SExpId> {
        self.visit_sexp(helper, unquote.unquoted)
    }
}

pub struct ThunkPass {}

struct StructVisitor;
impl Visitor for StructVisitor {
    fn visit_quasiquote(
        &mut self,
        helper: &mut PassHelper,
        quasiquote: Quasiquote,
    ) -> Option<SExpId> {
        quasiquote.visit_unquote(helper, self)
    }
    fn visit_list(&mut self, helper: &mut PassHelper, mut list: List) -> Option<SExpId> {
        println!("Visiting list: {:?}", helper.asts.fmt(list.id));
        if list.visit_children(helper, self) {
            Some(list.id)
        } else {
            None
        }
    }
}

impl ThunkPass {
    pub fn pass(asts: &mut ASTS, root: SExpId) -> SExpId {
        let mut helper = PassHelper::new(asts);
        let mut pass = Self {};
        pass.pass_inner(&mut helper, root).unwrap_or(root)
    }

    fn pass_inner(&mut self, helper: &mut PassHelper, id: SExpId) -> Option<SExpId> {
        let mut visitor = StructVisitor;
        visitor.visit_sexp(helper, id)
    }
}

impl PassHelper<'_> {
    //--

    fn new_ast(&mut self) -> &mut AST {
        self.asts.get_ast_by_generation(self.new_ast_id)
    }

    fn is_one_of_special_forms(&self, ids: &[SExpId], one_of: &[&str]) -> bool {
        let Some(first) = ids.first().copied() else {
            return false;
        };
        self.is_one_of(first, one_of)
    }

    fn is_one_of(&self, sexp_id: SExpId, symbols: &[&str]) -> bool {
        match self.asts.get(sexp_id) {
            SExp::Symbol(s) => symbols.contains(&s.as_str()),
            _ => false,
        }
    }

    // fn traverse(
    //     &mut self,
    //     id: SExpId,
    //     pre_order: &mut impl FnMut(&mut Self, &[SExpId]) -> bool,
    //     post_order: &mut impl FnMut(&mut Self, SExpId) -> Option<SExpId>,
    // ) -> Option<SExpId> {
    //     let sexp = self.asts.get(id);
    //     let mut sexp_ids = match sexp {
    //         SExp::List(sexp_ids) => sexp_ids.to_vec(),
    //         _ => {
    //             return post_order(self, id);
    //         }
    //     };
    //     if !pre_order(self, &sexp_ids) {
    //         return None;
    //     }
    //     let mut edited = false;
    //     for id in &mut sexp_ids {
    //         if let Some(new_id) = self.traverse(*id, pre_order, post_order) {
    //             *id = new_id;
    //             edited = true;
    //         }
    //     }
    //     post_order(self, id);
    //     if edited {
    //         Some(self.new_ast().add_node(SExp::List(sexp_ids)))
    //     } else {
    //         None
    //     }
    // }

    fn visit_mut_list<Pass>(
        &mut self,
        pass: &mut Pass,
        mut new_sexp_ids: Vec<SExpId>,
        mut f: impl FnMut(&mut Pass, &mut Self, SExpId) -> Option<SExpId>,
    ) -> Option<SExpId> {
        let mut edited = false;
        for id in &mut new_sexp_ids {
            if let Some(new_id) = f(pass, self, *id) {
                *id = new_id;
                edited = true;
            }
        }
        if edited {
            Some(self.new_ast().add_node(SExp::List(new_sexp_ids)))
        } else {
            None
        }
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
