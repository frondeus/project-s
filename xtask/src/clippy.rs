use super::*;

pub fn run(root: &Path) -> Result {
    run_command(
        "Clippy",
        root,
        "cargo",
        &[
            "clippy",
            "--all",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
    )
}
