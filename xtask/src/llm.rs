use crate::{run_command_q, Result};

use std::path::Path;

use crate::gen_syntax;

pub fn run(root: &Path) -> Result {
    run_command_q("Formatting", root, "cargo", &["fmt", "-q", "--all"], &[])?;
    run_command_q(
        "Clippy",
        root,
        "cargo",
        &[
            "clippy",
            "--color",
            "always",
            "-q",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
        &[],
    )?;

    gen_syntax::run(root)?;
    run_command_q(
        "Tree Sitter test",
        root.join("tree-sitter-s"),
        "tree-sitter",
        &["test"],
        &[],
    )?;
    let res = run_command_q(
        "Tests",
        root,
        "cargo",
        &["test", "--workspace", "--no-fail-fast", "-q", "--tests"],
        &[("TEST_RUNNER_Q", "1"), ("LLM_AGENT", "1")],
    );
    crate::review_tests::run(root, true)?;
    res?;
    Ok(())
}
