#![deny(clippy::print_stdout, clippy::print_stderr)]

use ast::{ASTS, SExpId};
// use diagnostics::Diagnostics;
use lambda_lifting::LambdaPass;
use runtime::Env;

pub mod cst;
pub mod source;

pub mod ast;
pub mod builder;

pub mod visitor;

pub mod lambda_lifting;

pub mod patterns;
pub mod types;

pub mod diagnostics;

pub mod modules;

pub mod api;

pub mod runtime;
pub use runtime::s_std;

pub mod lsp;

pub fn process_ast(asts: &mut ASTS, mut root: SExpId, envs: &[Env]) -> SExpId {
    root = LambdaPass::pass(asts, root, envs);
    // root = ThunkPass::pass(asts, root);

    // let mut type_env = types::TypeEnv::default().with_prelude();
    // let mut diagnostics = Diagnostics::default();
    // type_env.check(asts, root, &mut diagnostics);
    // if diagnostics.has_errors() {
    //     let p = diagnostics.pretty_print();
    //     tracing::error!("{}", p);
    //     // println!("{}", p);
    //     // panic!("ERROR: {}", p);
    // }

    root
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

        test_runner::test_snapshots_custom("docs/", "js", "js-eval", |input, _deps, _args| {
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
