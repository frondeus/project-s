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

pub mod graph;

pub fn process_ast(asts: &mut ASTS, root: SExpId, envs: &[Env]) -> (SExpId, Diagnostics) {
    let mut diagnostics = Diagnostics::default();
    // root = ThunkPass::pass(asts, root);
    let mut root = Spanned::new(root, asts.get(root).span);
    root = MacroExpansionPass::pass(asts, root, &mut diagnostics, envs);
    let mut root = root.inner();
    root = LambdaPass::pass(asts, root, envs);

    (root, diagnostics)
}

pub fn process_with_typechk<M: ModuleProvider>(
    mut modules: M,
    asts: &mut ASTS,
    root: SExpId,
    envs: &[Env],
) -> (SExpId, Diagnostics, M) {
    let (root, mut diagnostics) = process_ast(asts, root, envs);
    let mut type_env = types::TypeEnv::new().with_prelude(modules.sources_mut());
    type_env.infer(asts, root, &mut diagnostics, &mut modules);
    (root, diagnostics, modules)
}

#[cfg(test)]
pub fn level_from_args(
    args: &std::collections::HashSet<&str>,
) -> tracing::level_filters::LevelFilter {
    const LEVELS: &[(&str, tracing::level_filters::LevelFilter)] = &[
        ("trace", tracing::level_filters::LevelFilter::TRACE),
        ("debug", tracing::level_filters::LevelFilter::DEBUG),
        ("info", tracing::level_filters::LevelFilter::INFO),
        ("warn", tracing::level_filters::LevelFilter::WARN),
        ("error", tracing::level_filters::LevelFilter::ERROR),
    ];

    for (name, level) in LEVELS {
        if args.contains(name) {
            return *level;
        }
    }
    tracing::level_filters::LevelFilter::INFO
}

#[cfg(test)]
mod tests {
    // use super::*;

    use std::process::Command;

    use crate::{
        ast::ASTS,
        modules::{MemoryModules, ModuleProvider},
        runtime::Runtime,
        s_std::prelude,
        types::{InferedPolymorphicType, InferedTypeScheme},
    };

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

    #[test]
    fn eval() -> test_runner::Result {
        test_runner::test_snapshots("docs/", &["s"], "eval", |input, deps, args| {
            tracing::subscriber::with_default(tracing_subscriber::fmt().finish(), || {
                let lazy = args.contains("lazy");
                let (mut modules, source_id) = MemoryModules::from_deps(input, deps);
                let mut asts = ASTS::new();
                let source = modules.sources.get(source_id);
                let ast = asts.parse(source_id, source).unwrap();
                let root_id = ast.root_id().unwrap();
                let prelude = prelude();
                let envs = [prelude];
                let (root_id, mut diagnostics) = crate::process_ast(&mut asts, root_id, &envs);
                let mut type_env = crate::types::TypeEnv::new().with_prelude(modules.sources_mut());
                let infered = type_env.infer(&mut asts, root_id, &mut diagnostics, &mut modules);

                if diagnostics.has_errors() {
                    return diagnostics.pretty_print(modules.sources());
                }

                let mut runtime = Runtime::new(asts, Box::new(modules));
                let [prelude] = envs;
                runtime.with_env(prelude);

                let value = runtime.eval(root_id);
                let json = runtime.to_json(value, !lazy);

                let mut result = String::new();
                let t_env = type_env.top_env().clone();
                let r_env = runtime.top_env().clone();

                for (k, v) in t_env.iter() {
                    result.push_str("val ");
                    result.push_str(k);
                    result.push_str(" : ");
                    match v {
                        InferedTypeScheme::Monomorphic(infered_type_id) => {
                            let ty = type_env.coalesce(infered_type_id);
                            type_env.fmt(ty, &mut result).unwrap();
                        }
                        InferedTypeScheme::Polymorphic(InferedPolymorphicType { body, .. }) => {
                            let ty = type_env.coalesce(body);
                            result.push_str("forall ");
                            type_env.fmt(ty, &mut result).unwrap();
                        }
                    }
                    if let Some(json) = r_env.get(k) {
                        let json = runtime.to_json(json.clone(), !lazy);
                        result.push_str(" = ");
                        result.push_str(&serde_json::to_string_pretty(&json).unwrap());
                    }
                    result.push('\n');
                }

                let ty = type_env.coalesce(infered);
                result.push_str("- : ");
                type_env.fmt(ty, &mut result).unwrap();

                result.push_str(" = ");
                result.push_str(&serde_json::to_string_pretty(&json).unwrap());

                result
            })
        })
    }
}
