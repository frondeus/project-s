#![deny(clippy::print_stdout, clippy::print_stderr)]

use ast::{ASTS, SExpId};
use diagnostics::Diagnostics;
use lambda_lifting::LambdaPass;
use macro_expansion::MacroExpansionPass;
use modules::ModuleProvider;
use runtime::Env;

pub mod cst;
pub mod source;

pub mod ast;
pub mod builder;

pub mod visitor;

pub mod lambda_lifting;
pub mod macro_expansion;

pub mod patterns;
pub mod types;

pub mod diagnostics;

pub mod modules;

pub mod api;

pub mod runtime;
pub use runtime::s_std;

use crate::source::Spanned;

pub mod lsp;

pub fn process_ast(asts: &mut ASTS, root: SExpId, envs: &[Env]) -> (SExpId, Diagnostics) {
    let mut diagnostics = Diagnostics::default();
    // root = ThunkPass::pass(asts, root);
    let mut root = Spanned::new(root, asts.get(root).span);
    root = MacroExpansionPass::pass(asts, root, &mut diagnostics, envs);
    let mut root = root.inner();
    root = LambdaPass::pass(asts, root, envs);

    (root, diagnostics)
}

pub fn process_with_typechk(
    modules: impl ModuleProvider,
    asts: &mut ASTS,
    root: SExpId,
    envs: &[Env],
) -> (SExpId, Diagnostics, Box<dyn ModuleProvider>) {
    let (root, mut diagnostics) = process_ast(asts, root, envs);
    let mut type_env = types::TypeEnv::new(modules).with_prelude();
    type_env.check(asts, root, &mut diagnostics);
    let modules = type_env.finish();
    (root, diagnostics, modules)
}

#[cfg(test)]
mod tests {
    // use super::*;

    use std::process::Command;

    #[test]
    fn javascript() -> test_runner::Result {
        // This is NOT interopt. It is just
        // to evaluate js in documentation in order to compare it with
        // this language

        test_runner::test_snapshots("docs/", &["js"], "js-eval", |input, _deps, _args| {
            let output = Command::new("node")
                .args(["-p", input])
                .output()
                .expect("NodeJS");
            let mut result = String::from_utf8(output.stdout).unwrap();
            result.push_str(&String::from_utf8(output.stderr).unwrap());
            result.trim().to_string()
        })
    }
}
