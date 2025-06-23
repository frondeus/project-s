#![allow(dead_code)]
use std::collections::BTreeMap;

use crate::{
    ast::{ASTS, SExpId},
    runtime::Macro,
    visitor::{Visitor, VisitorHelper},
};

pub struct MacroExpansionPass<'a> {
    helper: VisitorHelper<'a>,
    envs: Envs,
}

impl<'a> MacroExpansionPass<'a> {
    pub fn pass(asts: &'a mut ASTS, root: SExpId) -> SExpId {
        let mut pass = Self {
            helper: VisitorHelper::new(asts),
            envs: Envs::default(),
        };
        pass.visit_sexp(root).unwrap_or(root)
    }
}

impl<'a> Visitor<'a> for MacroExpansionPass<'a> {
    fn helper_mut(&mut self) -> &mut VisitorHelper<'a> {
        &mut self.helper
    }

    fn helper(&self) -> &VisitorHelper<'a> {
        &self.helper
    }
}

#[derive(Default)]
struct Env {
    vars: BTreeMap<String, Macro>,
}

struct Envs {
    envs: Vec<Env>,
}

impl Default for Envs {
    fn default() -> Self {
        Self::new()
    }
}

impl Envs {
    fn new() -> Self {
        Self {
            envs: vec![Env::default()],
        }
    }

    pub fn push(&mut self) {
        self.envs.push(Env::default());
    }

    pub fn pop(&mut self) {
        self.envs.pop();
    }

    pub fn set(&mut self, name: &str, macro_: Macro) {
        self.envs
            .last_mut()
            .unwrap()
            .vars
            .insert(name.to_string(), macro_);
    }

    pub fn get(&self, name: &str) -> Option<&Macro> {
        self.envs.iter().rev().find_map(|env| env.vars.get(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macro_expansion() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "macro", |input, _deps, _args| {
            let mut asts = ASTS::new();
            let ast = asts.parse(input, "<input>").unwrap();
            let root_id = ast.root_id().unwrap();
            // let prelude = prelude();
            let new_root = MacroExpansionPass::pass(&mut asts, root_id);
            let output = asts.fmt(new_root);
            output.to_string()
        })
    }
}
