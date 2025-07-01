use std::{
    collections::{HashMap, hash_map::Entry},
    path::PathBuf,
};

use project_s::{
    ast::ASTS, lsp::Backend, modules::ModuleProvider, process_ast, runtime::Runtime,
    s_std::prelude, source::Sources, types::TypeEnv,
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

    let (mut sources, source_id) = Sources::single(filename, &document);
    let mut asts = ASTS::new();

    eprintln!("Parsing..");
    let Ok(ast) = asts.parse(source_id, sources.get(source_id)) else {
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
    let mut type_env = TypeEnv::default().with_prelude(&mut sources);
    eprintln!("Type checking..");
    type_env.check(&asts, root, &mut diagnostics);
    if diagnostics.has_errors() {
        eprintln!("Errors occurred during type checking");
        let err = diagnostics.pretty_print(&sources);
        eprintln!("{err}");
        return;
    }
    let modules = FileModules::from(sources);
    let mut runtime = Runtime::new(asts, Box::new(modules));
    runtime.with_prelude();
    eprintln!("Evaluating...");
    let value = runtime.eval(root);
    eprintln!("{:?}", value);
    eprintln!("Marshalling...");
    let json = runtime.to_json(value, true);
    let json = serde_json::to_string_pretty(&json).unwrap();
    print!("{json}")
}

struct FileModules {
    sources: Sources,
    path_to_id: HashMap<PathBuf, project_s::source::SourceId>,
}
impl From<Sources> for FileModules {
    fn from(sources: Sources) -> Self {
        let path_to_id = sources
            .iter_with_id()
            .map(|(id, src)| {
                let path = PathBuf::from(&*src.filename);
                (path, id)
            })
            .collect();
        Self {
            sources,
            path_to_id,
        }
    }
}
impl ModuleProvider for FileModules {
    fn get_module(&mut self, path: &std::path::Path) -> Option<project_s::source::SourceId> {
        let pathb = path.to_path_buf();
        match self.path_to_id.entry(pathb) {
            Entry::Occupied(entry) => Some(*entry.get()),
            Entry::Vacant(entry) => {
                let source = std::fs::read_to_string(path).ok()?;
                let path = path.display().to_string();
                let source_id = self.sources.add(&path, &source);
                entry.insert(source_id);
                Some(source_id)
            }
        }
    }

    fn get_source(
        &self,
        source_id: project_s::source::SourceId,
    ) -> Option<&project_s::source::Source> {
        Some(self.sources.get(source_id))
    }
}

async fn lsp() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
