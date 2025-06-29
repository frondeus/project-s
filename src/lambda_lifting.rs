use std::collections::BTreeSet;

use crate::{
    ast::{AST, ASTS, SExp, SExpId},
    builder::ASTBuilder,
    patterns::Pattern,
    source::{Span, Spanned},
};

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
    pub fn pass(asts: &'a mut ASTS, root: SExpId, envs: &'a [crate::runtime::Env]) -> SExpId {
        let new_ast = asts.new_ast();
        let ast_id = asts.add_ast(new_ast);
        let envs: Envs = envs.into();

        let mut pass = Self {
            envs,
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
        span: Span,
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
            Some(
                self.new_ast()
                    .add_node(SExp::List(new_sexp_ids), span, None),
            )
        } else {
            None
        }
    }

    fn process_do(
        &mut self,
        span: Span,
        sexp_ids: Vec<SExpId>,
        f: impl FnMut(&mut Self, SExpId) -> Option<SExpId>,
    ) -> Option<SExpId> {
        self.envs.push(EnvKind::Local);
        let result = self.visit_mut_list(span, sexp_ids, f);
        self.envs.pop();
        result
    }

    fn process_let(
        &mut self,
        span: Span,
        sexp_ids: Vec<SExpId>,
        f: impl FnMut(&mut Self, SExpId) -> Option<SExpId>,
    ) -> Option<SExpId> {
        // 0 - "let"
        // 1 - name
        // 2 - value
        let name = self.asts.get(sexp_ids[1]).as_keyword()?;
        self.envs.set(name);

        self.visit_mut_list(span, sexp_ids, f)
    }

    fn process_struct(
        &mut self,
        span: Span,
        sexp_ids: Vec<SExpId>,
        mut f: impl FnMut(&mut Self, SExpId) -> Option<SExpId>,
    ) -> Option<SExpId> {
        // println!("processing struct: {}", self.asts.fmt_list(&sexp_ids));
        self.envs.push(EnvKind::Object);
        // println!("processing struct: {}", self.asts.fmt_list(&sexp_ids));
        let result = self.process_struct_body(span, sexp_ids.to_vec(), |pass, id| f(pass, id));
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
        span: Span,
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
            // println!("Struct item: {}", self.asts.fmt(*id));
            if self.asts.get(*id).as_symbol_or_keyword().is_some() {
                // Key value pair
                if let Some(value) = list_iter.next() {
                    // println!(
                    //     "processing struct body key value: {}",
                    //     self.asts.fmt(*value)
                    // );
                    if let Some(new_id) = f(self, *value) {
                        *value = new_id;
                        edited = true;
                    }
                }
            }
        }
        if edited {
            Some(self.new_ast().add_node(SExp::List(list), span, None))
        } else {
            None
        }
        // })
    }

    fn pass_inner(&mut self, root: SExpId) -> Option<SExpId> {
        let sexp = self.asts.get(root);
        let span = sexp.span;
        let sexp_ids = sexp.as_list()?;
        let first_id = sexp_ids.first().copied()?;
        if self.is_one_of(first_id, &["quote", "quasiquote"]) {
            None
        } else if self.is_symbol(first_id, "do") {
            self.process_do(span, sexp_ids.to_vec(), |pass, id| pass.pass_inner(id))
        } else if self.is_one_of(first_id, &["let", "let-rec", "let*"]) {
            let sexp_ids = sexp_ids.to_vec();
            self.process_let(span, sexp_ids, |pass, id| pass.pass_inner(id))
        } else if self.is_symbol(first_id, "struct") {
            self.process_struct(span, sexp_ids.to_vec(), |pass, id| pass.pass_inner(id))
        } else if self.is_symbol(first_id, "thunk") {
            let free_vars = sexp_ids[1];
            let free_vars = self
                .asts
                .get(free_vars)
                .as_list()
                .unwrap()
                .iter()
                .map(|fv| self.spanned(*fv))
                .collect();
            let body = sexp_ids[2];
            let first_id = self.spanned(first_id);
            let body = self.spanned(body);
            self.process_thunk(first_id, free_vars, span, body)
        } else if self.is_symbol(first_id, "fn") {
            let first_id = self.spanned(first_id);
            let pattern_id = sexp_ids[1];
            let body = self.spanned(sexp_ids[2]);
            let pattern = Pattern::parse(pattern_id, self.asts).ok()?;
            self.process_fn(first_id, self.spanned(pattern_id), pattern, body, span)
        } else {
            self.visit_mut_list(span, sexp_ids.to_vec(), |pass, id| pass.pass_inner(id))
        }
    }

    fn process_thunk(
        &mut self,
        first_id: Spanned<SExpId>,
        free_vars: Vec<Spanned<SExpId>>,
        span: Span,
        mut body: Spanned<SExpId>,
    ) -> Option<SExpId> {
        self.envs.push(EnvKind::Function);

        let mut edited = false;
        if let Some(new_body) = self.pass_inner(body.inner()) {
            body = Spanned::new(new_body, body.span);
            edited = true;
        }
        let maybe_new_body = self.process_fn_decl(body);
        self.envs.pop();

        if let Some((new_body, new_free_vars)) = maybe_new_body {
            let mut free_vars = free_vars
                .into_iter()
                .map(|id| self.asts.get(id.inner()).as_symbol().unwrap().to_string())
                .collect::<BTreeSet<String>>();

            free_vars.extend(new_free_vars);
            // let free_vars = free_vars
            //     .into_iter()
            //     .map(|v| Spanned::new(v, span))
            //     .collect::<Vec<Spanned<String>>>();

            let thunk = (first_id, free_vars, new_body).assemble_id(self.new_ast(), span);

            Some(thunk)
        } else if edited {
            Some((first_id, free_vars, body).assemble_id(self.new_ast(), span))
        } else {
            None
        }
    }

    fn process_pattern(&mut self, pattern: Pattern) {
        match pattern {
            Pattern::Single(key, _) => {
                self.envs.set(&key);
            }
            Pattern::List(patterns, _) => {
                for pattern in patterns {
                    self.process_pattern(pattern);
                }
            }
            Pattern::Object(patterns, _) => {
                for (_key, pattern) in patterns {
                    self.process_pattern(pattern);
                }
            }
        }
    }

    fn process_fn(
        &mut self,
        first_id: Spanned<SExpId>,
        pattern_id: Spanned<SExpId>,
        pattern: Pattern,
        mut body: Spanned<SExpId>,
        span: Span,
    ) -> Option<SExpId> {
        self.envs.push(EnvKind::Function);

        self.process_pattern(pattern);

        let mut edited = false;
        if let Some(new_body) = self.pass_inner(body.inner()) {
            body = Spanned::new(new_body, body.span);
            edited = true;
        }
        let maybe_new_body = self.process_fn_decl(body);
        self.envs.pop();

        if let Some((new_body, free_vars)) = maybe_new_body {
            let cl = "cl".assemble_id(self.new_ast(), first_id.span);
            let closure = (cl, pattern_id, free_vars, new_body).assemble_id(self.new_ast(), span);

            Some(closure)
        } else if edited {
            Some((first_id, pattern_id, body).assemble_id(self.new_ast(), span))
        } else {
            None
        }
    }

    fn process_fn_decl(
        &mut self,
        body: Spanned<SExpId>,
    ) -> Option<(Spanned<SExpId>, BTreeSet<String>)> {
        // println!("processing fn decl: {}", self.asts.fmt(body));
        let mut free_vars = BTreeSet::<String>::new();

        let new_body = self
            .process_fn_decl_body(body.inner(), &mut free_vars)
            .map(|id| Spanned::new(id, body.span));

        if free_vars.is_empty() {
            None
        } else {
            Some((new_body.unwrap_or(body), free_vars))
        }
    }

    fn process_quasiquote(
        &mut self,
        span: Span,
        sexp_ids: Vec<SExpId>,
        free_vars: &mut BTreeSet<String>,
    ) -> Option<SExpId> {
        // println!("processing quasiquote: {}", self.asts.fmt_list(&sexp_ids));
        self.visit_mut_list(span, sexp_ids.clone(), |pass, id| {
            let sexp = pass.asts.get(id);
            let span = sexp.span;
            if let Some(list) = sexp.as_list() {
                let first = list[0];
                let list = list.to_vec();
                if pass.is_symbol(first, "unquote") {
                    if let Some(new_id) = pass.process_unquote(span, list.clone(), free_vars) {
                        return Some(new_id);
                    }
                } else if let Some(new_id) = pass.process_quasiquote(span, list, free_vars) {
                    return Some(new_id);
                }
            }
            None
        })
    }

    fn process_unquote(
        &mut self,
        span: Span,
        sexp_ids: Vec<SExpId>,
        free_vars: &mut BTreeSet<String>,
    ) -> Option<SExpId> {
        // println!("processing unquote: {}", self.asts.fmt(sexp_ids[0]));
        self.visit_mut_list(span, sexp_ids.clone(), |pass, id| {
            pass.process_fn_decl_body(id, free_vars)
        })
    }

    fn process_captured(&mut self, id: SExpId, free_vars: &mut BTreeSet<String>) -> Option<SExpId> {
        let sexp = self.asts.get(id);
        let span = sexp.span;
        let list = sexp.as_list()?;
        self.visit_mut_list(span, list.to_vec(), |pass, id| {
            pass.process_fn_decl_body(id, free_vars)
        })
    }

    fn process_fn_decl_body(
        &mut self,
        body: SExpId,
        free_vars: &mut BTreeSet<String>,
    ) -> Option<SExpId> {
        let sexp = self.asts.get(body);
        let span = sexp.span;
        match &**sexp {
            SExp::Symbol(s) if SPECIAL_FORMS.contains(&s.as_str()) => None,
            SExp::Symbol(s) => match self.envs.has(s) {
                Some(VariableKind::Free) => {
                    tracing::trace!("free var: {}", s);
                    free_vars.insert(s.clone());

                    None
                }

                None | Some(VariableKind::Local) => None,
            },
            SExp::List(ids) if ids.is_empty() => None,
            SExp::List(sexp_ids) => {
                let first = sexp_ids[0];
                if self.is_symbol(first, "quote") {
                    return None;
                }
                if self.is_symbol(first, "quasiquote") {
                    return self.process_quasiquote(span, sexp_ids.clone(), free_vars);
                }
                if self.is_symbol(first, "do") {
                    return self.process_do(span, sexp_ids.clone(), move |pass, id| {
                        pass.process_fn_decl_body(id, free_vars)
                    });
                }
                if self.is_one_of(first, &["let", "let-rec", "let*"]) {
                    let sexp_ids = sexp_ids.to_vec();
                    return self.process_let(span, sexp_ids, move |pass, id| {
                        pass.process_fn_decl_body(id, free_vars)
                    });
                }
                if self.is_symbol(first, "struct") {
                    return self.process_struct(span, sexp_ids.clone(), move |pass, id| {
                        pass.process_fn_decl_body(id, free_vars)
                    });
                }
                if self.is_symbol(first, "thunk") {
                    let captured = sexp_ids[1];
                    return self.process_captured(captured, free_vars);
                }
                if self.is_symbol(first, "fn") {
                    return None;
                }
                if self.is_symbol(first, "cl") {
                    let _signature = sexp_ids[1];
                    let captured = sexp_ids[2];
                    return self.process_captured(captured, free_vars);
                }

                self.visit_mut_list(span, sexp_ids.clone(), |pass, id| {
                    pass.process_fn_decl_body(id, free_vars)
                })
            }
            _ => None,
        }
    }

    fn is_symbol(&self, sexp_id: SExpId, symbol: &str) -> bool {
        self.asts.get(sexp_id).as_symbol() == Some(symbol)
    }

    fn is_one_of(&self, sexp_id: SExpId, symbols: &[&str]) -> bool {
        self.asts
            .get(sexp_id)
            .as_symbol()
            .is_some_and(|s| symbols.contains(&s))
    }

    fn spanned(&self, sexp_id: SExpId) -> Spanned<SExpId> {
        Spanned::new(sexp_id, self.asts.get(sexp_id).span)
    }
}

impl From<&crate::runtime::Env> for Env {
    fn from(env: &crate::runtime::Env) -> Self {
        Self {
            // Problem: This populates only with latest env.
            vars: env.keys().map(|k| k.to_string()).collect(),
            kind: EnvKind::Global,
        }
    }
}
impl From<&[crate::runtime::Env]> for Envs {
    fn from(envs: &[crate::runtime::Env]) -> Self {
        let mut envs_iter = envs.iter();
        let mut envs = Vec::new();

        if let Some(global) = envs_iter.next() {
            envs.push(Env {
                vars: global.keys().map(|k| k.to_string()).collect(),
                kind: EnvKind::Global,
            });
        };

        for env in envs_iter {
            envs.push(Env {
                vars: env.keys().map(|k| k.to_string()).collect(),
                kind: EnvKind::Local,
            });
        }

        Self { envs }
    }
}

#[derive(Debug)]
struct Env {
    vars: BTreeSet<String>,
    kind: EnvKind,
}

#[derive(Debug)]
struct Envs {
    envs: Vec<Env>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
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
            if env.vars.contains(name) && env.kind != EnvKind::Global {
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
    use crate::{s_std::prelude, source::Sources};

    use super::*;

    #[test]
    fn lift() -> test_runner::Result {
        test_runner::test_snapshots("docs/", &["", "s"], "lift", |input, _deps, _args| {
            let mut asts = ASTS::new();
            let (sources, source_id) = Sources::single("<input>", input);
            let ast = asts.parse(source_id, sources.get(source_id)).unwrap();
            let root_id = ast.root_id().unwrap();
            let prelude = prelude();
            let new_root = LambdaPass::pass(&mut asts, root_id, &[prelude]);
            let output = asts.fmt(new_root);
            output.to_string()
        })
    }
}
