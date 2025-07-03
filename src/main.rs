use project_s::{
    ast::ASTS,
    lsp::Backend,
    modules::{FileModules, ModuleProvider},
    process_ast,
    runtime::Runtime,
    s_std::prelude,
    source::Sources,
    types::TypeEnv,
};
use tower_lsp_server::{LspService, Server};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args = std::env::args().collect::<Vec<_>>();
    let args = args.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    match &args[1..] {
        ["lsp", ..] => lsp().await,
        ["run", input] => run(input),
        _ => lsp().await,
    }
}

fn run(filename: &str) {
    let document = std::fs::read_to_string(filename).unwrap();

    let (sources, source_id) = Sources::single(filename, &document);
    let mut asts = ASTS::new();
    let modules = FileModules::from(sources);

    eprintln!("Parsing..");
    let Ok(ast) = asts.parse(source_id, modules.sources().get(source_id)) else {
        eprintln!("Could not parse file: {filename}");
        return;
    };
    let Some(root) = ast.root_id() else {
        eprintln!("Could not get parsed root SEXP: {filename}");
        return;
    };
    let prelude = prelude();
    let envs = &[prelude];
    eprintln!("Processing..");
    let (root, mut diagnostics) = process_ast(&mut asts, root, envs);
    let mut type_env = TypeEnv::new(modules).with_prelude();
    eprintln!("Type checking..");
    type_env.check(&mut asts, root, &mut diagnostics);
    let modules = type_env.finish();
    if diagnostics.has_errors() {
        eprintln!("Errors occurred during type checking");
        let err = diagnostics.pretty_print(modules.sources());
        eprintln!("{err}");
        return;
    }
    let mut runtime = Runtime::new(asts, modules);
    runtime.with_prelude();
    eprintln!("Evaluating...");
    let value = runtime.eval(root);
    eprintln!("{:?}", value);
    eprintln!("Marshalling...");
    let json = runtime.to_json(value, true);
    let json = serde_json::to_string_pretty(&json).unwrap();
    print!("{json}")
}

async fn lsp() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
