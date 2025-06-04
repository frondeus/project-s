#![allow(dead_code)]
use std::collections::BTreeSet;

use crate::{
    ast::{AST, ASTS, SExp, SExpId},
    builder::ASTBuilder,
};

pub const CLOSURE_SYMBOL: &str = "$$closure";
const SPECIAL_FORMS: &[&str] = &[
    "quasiquote",
    "+",
    "unquote",
    "let",
    "fn",
    "cl",
    "struct",
    "is-type",
    "quote",
    "has?",
];

pub struct LambdaPass<'a> {
    envs: Envs,
    asts: &'a mut ASTS,
    new_ast_id: usize,
}

impl<'a> LambdaPass<'a> {
    pub fn pass(asts: &'a mut ASTS, root: SExpId) -> SExpId {
        let new_ast = asts.new_ast();
        let ast_id = asts.add_ast(new_ast);
        let mut pass = Self {
            envs: Envs::new(),
            asts,
            new_ast_id: ast_id,
        };
        pass.pass_inner(root).unwrap_or(root)
    }

    fn new_ast(&mut self) -> &mut AST {
        self.asts.get_ast_by_generation(self.new_ast_id)
    }

    fn visit_mut_list(
        &mut self,
        mut new_sexp_ids: Vec<SExpId>,
        mut f: impl FnMut(&mut Self, SExpId) -> Option<SExpId>,
    ) -> Option<SExpId> {
        let mut edited = false;
        for id in &mut new_sexp_ids {
            if let Some(new_id) = f(self, *id) {
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

    fn process_do(
        &mut self,
        sexp_ids: Vec<SExpId>,
        f: impl FnMut(&mut Self, SExpId) -> Option<SExpId>,
    ) -> Option<SExpId> {
        self.envs.push(EnvKind::Local);
        let result = self.visit_mut_list(sexp_ids, f);
        self.envs.pop();
        result
    }

    fn process_let(
        &mut self,
        sexp_ids: Vec<SExpId>,
        f: impl FnMut(&mut Self, SExpId) -> Option<SExpId>,
    ) -> Option<SExpId> {
        // 0 - "let"
        // 1 - name
        // 2 - value
        let name = self.asts.get(sexp_ids[1]).as_keyword()?;
        self.envs.set(name);

        self.visit_mut_list(sexp_ids, f)
    }

    fn process_struct(
        &mut self,
        sexp_ids: Vec<SExpId>,
        mut f: impl FnMut(&mut Self, SExpId) -> Option<SExpId>,
    ) -> Option<SExpId> {
        // println!("processing struct: {}", self.asts.fmt_list(&sexp_ids));
        self.envs.push(EnvKind::Object);
        println!("processing struct: {}", self.asts.fmt_list(&sexp_ids));
        let result = self.process_struct_body(sexp_ids.to_vec(), |pass, id| f(pass, id));
        self.envs.pop();
        result

        // let result = self.visit_mut_list(sexp_ids, |pass, id| {
        //     let list = pass.asts.get(id).as_list()?;

        //     let first = list[0];
        //     if !pass.is_symbol(first, "quote") {
        //         return None;
        //     }

        //     pass.process_struct_body(list.to_vec(), |pass, id| f(pass, id))
        // });
        // self.envs.pop();
        // result
    }

    fn process_struct_body(
        &mut self,
        mut list: Vec<SExpId>,
        mut f: impl FnMut(&mut Self, SExpId) -> Option<SExpId>,
    ) -> Option<SExpId> {
        // self.visit_mut_list(sexp_ids, |pass, id| {
        //     let mut list = pass.asts.get(id).as_list()?.to_vec();
        //     // println!("processing struct body: {}", pass.asts.fmt_list(&list));
        let mut list_iter = list.iter_mut();
        let mut edited = false;
        list_iter.next(); // Skip struct keyword
        while let Some(id) = list_iter.next() {
            println!("Struct item: {}", self.asts.fmt(*id));
            if self.asts.get(*id).as_symbol_or_keyword().is_some() {
                // Key value pair
                if let Some(value) = list_iter.next() {
                    println!(
                        "processing struct body key value: {}",
                        self.asts.fmt(*value)
                    );
                    // println!(
                    //     "processing struct body key value: {}",
                    //     pass.asts.fmt(*value)
                    // );
                    if let Some(new_id) = f(self, *value) {
                        *value = new_id;
                        edited = true;
                    }
                }
            }
        }
        if edited {
            Some(self.new_ast().add_node(SExp::List(list)))
        } else {
            None
        }
        // })
    }

    fn pass_inner(&mut self, root: SExpId) -> Option<SExpId> {
        if let SExp::List(sexp_ids) = self.asts.get(root) {
            let first_id = sexp_ids.first().copied()?;
            if self.is_one_of(first_id, &["quote", "quasiquote"]) {
                None
            } else if self.is_symbol(first_id, "do") {
                self.process_do(sexp_ids.to_vec(), |pass, id| pass.pass_inner(id))
            } else if self.is_symbol(first_id, "let") {
                let sexp_ids = sexp_ids.to_vec();
                self.process_let(sexp_ids, |pass, id| pass.pass_inner(id))
            } else if self.is_symbol(first_id, "struct") {
                self.process_struct(sexp_ids.to_vec(), |pass, id| pass.pass_inner(id))
            } else if self.is_symbol(first_id, "fn") {
                // println!("processing fn: {}", self.asts.fmt_list(sexp_ids));
                let signature_id = sexp_ids[1];
                let signature = self.asts.get(signature_id).as_list().unwrap().to_vec();
                let signature = signature
                    .iter()
                    .map(|id| self.asts.get(*id).as_keyword().unwrap().to_string())
                    .collect::<Vec<String>>();
                self.envs.push(EnvKind::Function);
                for var in signature {
                    self.envs.set(&var);
                }

                let mut body = sexp_ids[2];

                let mut edited = false;
                if let Some(new_body) = self.pass_inner(body) {
                    body = new_body;
                    edited = true;
                }
                let maybe_new_body = self.process_fn_decl(body);
                self.envs.pop();

                if let Some((new_body, free_vars)) = maybe_new_body {
                    let closure =
                        ("cl", signature_id, free_vars, new_body).assemble(self.new_ast());

                    Some(closure)
                } else if edited {
                    Some(
                        self.new_ast()
                            .add_node(SExp::List(vec![first_id, signature_id, body])),
                    )
                } else {
                    None
                }
            } else {
                self.visit_mut_list(sexp_ids.clone(), |pass, id| pass.pass_inner(id))
            }
        } else {
            None
        }
    }

    fn process_fn_decl(&mut self, body: SExpId) -> Option<(SExpId, BTreeSet<String>)> {
        println!("processing fn decl: {}", self.asts.fmt(body));
        let mut free_vars = BTreeSet::<String>::new();

        let body = self.process_fn_decl_body(body, &mut free_vars);
        body.map(|id| (id, free_vars))
    }

    fn process_quasiquote(
        &mut self,
        sexp_ids: Vec<SExpId>,
        free_vars: &mut BTreeSet<String>,
    ) -> Option<SExpId> {
        // println!("processing quasiquote: {}", self.asts.fmt_list(&sexp_ids));
        self.visit_mut_list(sexp_ids.clone(), |pass, id| {
            if let Some(list) = pass.asts.get(id).as_list() {
                let first = list[0];
                let list = list.to_vec();
                if pass.is_symbol(first, "unquote") {
                    if let Some(new_id) = pass.process_unquote(list.clone(), free_vars) {
                        return Some(new_id);
                    }
                } else if let Some(new_id) = pass.process_quasiquote(list, free_vars) {
                    return Some(new_id);
                }
            }
            None
        })
    }

    fn process_unquote(
        &mut self,
        sexp_ids: Vec<SExpId>,
        free_vars: &mut BTreeSet<String>,
    ) -> Option<SExpId> {
        // println!("processing unquote: {}", self.asts.fmt(sexp_ids[0]));
        self.visit_mut_list(sexp_ids.clone(), |pass, id| {
            pass.process_fn_decl_body(id, free_vars)
        })
    }

    fn process_fn_decl_body(
        &mut self,
        body: SExpId,
        free_vars: &mut BTreeSet<String>,
    ) -> Option<SExpId> {
        match self.asts.get(body) {
            SExp::Symbol(s) if SPECIAL_FORMS.contains(&s.as_str()) => None,
            SExp::Symbol(s) => match self.envs.has(s) {
                Some(VariableKind::Free) => {
                    println!("free var: {}", s);
                    free_vars.insert(s.clone());
                    let s = format!(":{s}");
                    let id = (CLOSURE_SYMBOL, s).assemble(self.new_ast());

                    Some(id)
                }

                None | Some(VariableKind::Local) => None,
            },
            SExp::List(sexp_ids) => {
                let first = sexp_ids[0];
                if self.is_symbol(first, "quote") {
                    return None;
                }
                if self.is_symbol(first, "quasiquote") {
                    return self.process_quasiquote(sexp_ids.clone(), free_vars);
                }
                if self.is_symbol(first, "do") {
                    return self.process_do(sexp_ids.clone(), move |pass, id| {
                        pass.process_fn_decl_body(id, free_vars)
                    });
                }
                if self.is_symbol(first, "let") {
                    let sexp_ids = sexp_ids.to_vec();
                    return self.process_let(sexp_ids, move |pass, id| {
                        pass.process_fn_decl_body(id, free_vars)
                    });
                }
                if self.is_symbol(first, "struct") {
                    return self.process_struct(sexp_ids.clone(), move |pass, id| {
                        pass.process_fn_decl_body(id, free_vars)
                    });
                }
                if self.is_symbol(first, "fn") {
                    return None;
                }
                if self.is_symbol(first, "cl") {
                    return None;
                }

                self.visit_mut_list(sexp_ids.clone(), |pass, id| {
                    pass.process_fn_decl_body(id, free_vars)
                })
            }
            _ => None,
        }
    }

    fn is_symbol(&self, sexp_id: SExpId, symbol: &str) -> bool {
        match self.asts.get(sexp_id) {
            SExp::Symbol(s) => s == symbol,
            _ => false,
        }
    }

    fn is_one_of(&self, sexp_id: SExpId, symbols: &[&str]) -> bool {
        match self.asts.get(sexp_id) {
            SExp::Symbol(s) => symbols.contains(&s.as_str()),
            _ => false,
        }
    }
}

struct Env {
    vars: BTreeSet<String>,
    kind: EnvKind,
}

struct Envs {
    envs: Vec<Env>,
}

#[derive(Clone, Copy, PartialEq)]
enum EnvKind {
    Global,
    Function,
    Object,
    Local,
}

impl Default for Envs {
    fn default() -> Self {
        Self::new()
    }
}

impl Env {
    fn new(kind: EnvKind) -> Self {
        Self {
            vars: BTreeSet::new(),
            kind,
        }
    }

    fn global() -> Self {
        Self::new(EnvKind::Global)
    }
}

#[derive(Debug, Clone, Copy)]
enum VariableKind {
    Local,
    Free,
}

impl Envs {
    pub fn new() -> Self {
        Self {
            envs: vec![Env::global()],
        }
    }

    pub fn push(&mut self, kind: EnvKind) {
        self.envs.push(Env::new(kind));
    }

    pub fn pop(&mut self) {
        self.envs.pop();
    }

    fn last_mut(&mut self) -> &mut Env {
        self.envs.last_mut().expect("No environment")
    }

    pub fn set(&mut self, name: &str) {
        self.last_mut().vars.insert(name.to_string());
    }

    pub fn has(&self, name: &str) -> Option<VariableKind> {
        let mut outcome = VariableKind::Local;
        const OBJECT_RELATED_VARS: &[&str] = &["self", "super", "root"];
        for env in self.envs.iter().rev() {
            if env.kind == EnvKind::Object && OBJECT_RELATED_VARS.contains(&name) {
                return Some(outcome);
            }
            if env.vars.contains(name) {
                return Some(outcome);
            }
            if let EnvKind::Function = env.kind {
                outcome = VariableKind::Free;
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
        test_runner::test_snapshots("docs/", "lift", |input, _deps| {
            eprintln!("---");
            let mut asts = ASTS::new();
            let ast = asts.parse(input).unwrap();
            let root_id = ast.root_id().unwrap();
            let new_root = LambdaPass::pass(&mut asts, root_id);
            let output = asts.fmt(new_root);
            output.to_string()
        })
    }
}
