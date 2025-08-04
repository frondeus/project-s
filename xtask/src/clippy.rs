use super::*;

pub fn run(root: &Path) -> Result {
    run_command(
        "Clippy",
        root,
        "cargo",
        &[
            "clippy",
            "-q",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
    )
}
