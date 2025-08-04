use super::*;

pub fn run(root: &Path) -> Result {
    run_command("Formatting", root, "cargo", &["fmt", "--all", "-q"])
}
