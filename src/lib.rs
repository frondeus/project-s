#![deny(clippy::print_stdout, clippy::print_stderr)]

use ast::{ASTS, SExpId};
use lambda_lifting::LambdaPass;
use runtime::Env;

pub mod ast;
pub mod builder;

pub mod visitor;

pub mod lambda_lifting;

pub mod types;

pub mod runtime;
pub use runtime::s_std;

pub fn process_ast(asts: &mut ASTS, mut root: SExpId, envs: &[Env]) -> SExpId {
    root = LambdaPass::pass(asts, root, envs);
    // root = ThunkPass::pass(asts, root);
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
