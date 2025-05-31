#![allow(dead_code)]
use std::collections::BTreeSet;

use crate::ast::{AST, ASTS, SExp, SExpId};

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
        let new_ast = AST::default();
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

    fn process_let(
        &mut self,
        sexp_ids: Vec<SExpId>,
        f: impl FnMut(&mut Self, SExpId) -> Option<SExpId>,
    ) -> Option<SExpId> {
        // 0 - "let"
        // 1 - name
        // 2 - value
        // 3 - body
        let name = self.asts.get(sexp_ids[1]).as_symbol().unwrap();
        self.envs.push(EnvKind::Local);
        self.envs.set(name);

        let result = self.visit_mut_list(sexp_ids, f);
        self.envs.pop();
        result
    }

    fn pass_inner(&mut self, root: SExpId) -> Option<SExpId> {
        if let SExp::List(sexp_ids) = self.asts.get(root) {
            let first_id = sexp_ids[0];
            if self.is_one_of(first_id, &["quote", "quasiquote"]) {
                None
            } else if self.is_symbol(first_id, "let") {
                let sexp_ids = sexp_ids.to_vec();
                self.process_let(sexp_ids, |pass, id| pass.pass_inner(id))
            } else if self.is_symbol(first_id, "fn") {
                let signature_id = sexp_ids[1];
                let signature = self.asts.get(signature_id).as_list().unwrap().to_vec();
                let signature = signature
                    .iter()
                    .map(|id| self.asts.get(*id).as_symbol().unwrap().to_string())
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
                    let id = self.new_ast().reserve();
                    let closure_symbol = self.new_ast().add_node(SExp::Symbol("cl".to_string()));
                    let captured_id = self.new_ast().reserve();

                    let free_vars = free_vars
                        .into_iter()
                        .map(|v| self.new_ast().add_node(SExp::Symbol(v)))
                        .collect();

                    self.new_ast().set(captured_id, SExp::List(free_vars));

                    self.new_ast().set(
                        id,
                        SExp::List(vec![closure_symbol, signature_id, captured_id, new_body]),
                    );
                    Some(id)
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
        // println!("processing fn decl: {}", self.asts.fmt(body));
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
                    free_vars.insert(s.clone());
                    let s = format!(":{s}");
                    let id = self.new_ast().reserve();
                    let closure = self
                        .new_ast()
                        .add_node(SExp::Symbol(CLOSURE_SYMBOL.to_string()));
                    let symbol = self.new_ast().add_node(SExp::Symbol(s));
                    self.new_ast().set(id, SExp::List(vec![closure, symbol]));

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
                if self.is_symbol(first, "let") {
                    let sexp_ids = sexp_ids.to_vec();
                    return self.process_let(sexp_ids, move |pass, id| {
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

pub fn lift_lambdas(asts: &mut ASTS, root: SExpId) -> SExpId {
    LambdaPass::pass(asts, root)
}

struct Env {
    vars: BTreeSet<String>,
    kind: EnvKind,
}

struct Envs {
    envs: Vec<Env>,
}

#[derive(Clone, Copy)]

enum EnvKind {
    Global,
    Function,
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
        for env in self.envs.iter().rev() {
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
            let ast = crate::ast::AST::parse(input).unwrap();
            let root_id = ast.root_id().unwrap();
            let mut asts = ASTS::new(ast);
            let new_root = lift_lambdas(&mut asts, root_id);
            let output = asts.fmt(new_root);
            output.to_string()
        })
    }
}
