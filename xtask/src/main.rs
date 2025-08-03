use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

mod clippy;
mod fmt;
mod gen_syntax;
mod helix;
mod llm;
mod repl;
mod review_tests;
mod test;
mod zed;

fn main() {
    if let Err(e) = try_main() {
        eprintln!("Error: {e}");
        std::process::exit(-1);
    }
}

type DynError = Box<dyn std::error::Error>;
type Result<T = (), E = DynError> = std::result::Result<T, E>;

fn try_main() -> Result {
    let args = env::args().skip(1).collect::<Vec<String>>();
    let args = args.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
    let root = project_root();
    match args[..] {
        ["gen-syntax"] | ["gs"] => gen_syntax::run(&root)?,
        ["review-tests"] | ["rt"] => review_tests::run(&root, false)?,
        ["test"] | ["t"] => test::run(&root)?,
        ["repl"] | ["r"] => repl::run(&root)?,
        ["clippy"] | ["cl"] => clippy::run(&root)?,
        ["fmt"] | ["f"] => fmt::run(&root)?,
        ["helix"] | ["hx"] => helix::run(&root)?,
        ["zed"] => zed::run(&root)?,
        ["llm"] => llm::run(&root)?,
        ["ci"] => {
            fmt::run(&root)?;
            clippy::run(&root)?;
            gen_syntax::run(&root)?;
            let res = test::run(&root);
            review_tests::run(&root, false)?;
            res?;
        }
        _ => print_help(),
    }
    Ok(())
}

fn print_help() {
    eprintln!(
        "Tasks:
        gen-syntax [gs] - Generate TreeSitter parser
        review-tests [rt] - Review snapshot tests
        test [t] - Run all tests (including TreeSitter tests)
        clippy [cl] - Run cargo clippy
        fmt [f] - Run cargo fmt

        repl [r] - Run REPL
        helix [hx] - Build grammar for helix editor
        zed - Build grammar for zed editor

        llm - CI for LLM Agents

        ci - ['gen-syntax', 'test', 'review-tests']
    "
    );
}

pub fn project_root() -> PathBuf {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    cargo_path.parent().unwrap().to_path_buf()
}

pub fn run_command_q(
    desc: &str,
    dir: impl AsRef<Path>,
    cmd: &str,
    args: &[&str],
    envs: &[(&str, &str)],
) -> Result {
    print!("{desc}...");
    let mut cmd = Command::new(cmd);
    cmd.current_dir(dir).args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let output = cmd.output()?;

    if !output.status.success() {
        println!(" x");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        println!("{stdout}");
        eprintln!("{stderr}");

        Err(format!("{desc} failed"))?;
    }
    println!(" v");
    Ok(())
}

pub fn run_command(desc: &str, dir: impl AsRef<Path>, cmd: &str, args: &[&str]) -> Result {
    print!("{desc}...");
    let status = Command::new(cmd).current_dir(dir).args(args).status()?;

    if !status.success() {
        println!(" x");
        Err(format!("{desc} failed"))?;
    }
    println!(" v");
    Ok(())
}
