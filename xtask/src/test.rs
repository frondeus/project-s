use std::path::Path;

use crate::{run_command, Result};

pub fn run(root: &Path) -> Result {
    run_command(
        "tree-sitter test",
        root.join("tree-sitter-s"),
        "tree-sitter",
        &["test"],
    )?;

    run_command(
        "cargo test",
        root,
        "cargo",
        &["test", "--all", "--no-fail-fast", "-q", "--tests"],
    )?;

    Ok(())
}
