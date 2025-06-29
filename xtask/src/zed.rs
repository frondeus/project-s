use crate::Result;

use std::path::{Path, PathBuf};

pub fn run(root: &Path) -> Result {
    // let home = std::env::var("HOME")?;
    let queries_to_copy = [
        "highlights.scm",
        "brackets.scm",
        // "locals.scm",
        // "textobjects.scm",
        // "indents.scm",
    ];
    for query in queries_to_copy {
        let source = root.join("tree-sitter-s/queries/").join(query);
        let target = PathBuf::from(&root)
            .join("zed/languages/project_s/")
            .join(query);

        println!("Copying {source:?} to {target:?}");
        std::fs::copy(source, target)?;
    }
    std::fs::remove_dir_all(root.join("zed/grammars"))?;
    crate::clippy::run(root)?;
    crate::gen_syntax::run(root)?;
    // run_command("hx fetch queries", root, "hx", &["-g", "fetch"])?;
    // run_command("hx fetch queries", root, "hx", &["-g", "build"])?;

    Ok(())
}
