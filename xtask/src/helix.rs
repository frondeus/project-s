use crate::{run_command, Result};

use std::path::{Path, PathBuf};

pub fn run(root: &Path) -> Result {
    let home = std::env::var("HOME")?;
    let queries_to_copy = [
        "highlights.scm",
        "locals.scm",
        "textobjects.scm",
        "indents.scm",
    ];
    for query in queries_to_copy {
        let source = root.join("tree-sitter-s/queries/").join(query);
        let target = PathBuf::from(&home)
            .join(".config/helix/runtime/queries/s/")
            .join(query);

        println!("Copying {source:?} to {target:?}");
        std::fs::copy(source, target)?;
    }
    crate::clippy::run(root)?;
    crate::gen_syntax::run(root)?;
    run_command("hx fetch queries", root, "hx", &["-g", "fetch"])?;
    run_command("hx fetch queries", root, "hx", &["-g", "build"])?;

    Ok(())
}
