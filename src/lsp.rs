// use std::{
//     path::{Path, PathBuf},
//     sync::Arc,
// };

use std::str::FromStr;

use highlights::lsp_legend;
// use tokio::sync::Mutex;
use tower_lsp_server::{
    Client, LanguageServer, UriExt,
    jsonrpc::Result,
    lsp_types::{
        Diagnostic, DiagnosticOptions, DiagnosticRelatedInformation, DiagnosticServerCapabilities,
        DiagnosticSeverity, DidChangeConfigurationParams, DidChangeTextDocumentParams,
        DidChangeWatchedFilesParams, DidChangeWorkspaceFoldersParams, DidCloseTextDocumentParams,
        DidOpenTextDocumentParams, DidSaveTextDocumentParams, DocumentDiagnosticParams,
        DocumentDiagnosticReport, DocumentDiagnosticReportResult, FullDocumentDiagnosticReport,
        Hover, HoverContents, HoverParams, HoverProviderCapability, InitializeParams,
        InitializeResult, InitializedParams, Location, MarkupContent, MarkupKind, MessageType,
        OneOf, Position, RelatedFullDocumentDiagnosticReport, SemanticTokens,
        SemanticTokensFullOptions, SemanticTokensOptions, SemanticTokensParams,
        SemanticTokensResult, SemanticTokensServerCapabilities, ServerCapabilities,
        TextDocumentSyncCapability, TextDocumentSyncKind, Uri, WorkspaceFoldersServerCapabilities,
        WorkspaceServerCapabilities,
    },
};
use tree_sitter::Point;

fn pos_to_point(pos: Position) -> Point {
    let Position { line, character } = pos;
    Point {
        row: line as usize,
        column: character as usize,
    }
}

fn point_to_pos(point: Point) -> Position {
    Position {
        line: point.row as u32,
        character: point.column as u32,
    }
}

fn ts_range_to_range(range: tree_sitter::Range) -> tower_lsp_server::lsp_types::Range {
    let start = point_to_pos(range.start_point);
    let end = point_to_pos(range.end_point);
    tower_lsp_server::lsp_types::Range { start, end }
}

use crate::{
    ast::ASTS,
    modules::{FileModules, ModuleProvider},
    process_ast,
    s_std::prelude,
    source::{SourceId, Sources},
    types::TypeEnv,
};

mod highlights;

// #[derive(Default)]
// pub struct LSPSources {
//     sources: Arc<Mutex<Sources>>,
// }

// impl LSPSources {
//     async fn load(&self, path: impl AsRef<Path>) {
//         let mut sources = self.sources.lock().await;
//         let filename = path.as_ref().display().to_string();
//         let source =
//             sources.find_or_load_with(&filename, || std::fs::read_to_string(path).expect("File"));
//     }
// }

pub struct Backend {
    client: Client,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        self.client
            .log_message(MessageType::INFO, "Initializing language server")
            .await;

        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),

                hover_provider: Some(HoverProviderCapability::Simple(true)),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        ..Default::default()
                    },
                )),

                // completion_provider: Some(CompletionOptions {
                //     resolve_provider: Some(false),
                //     trigger_characters: Some(vec![".".to_string()]),
                //     work_done_progress_options: Default::default(),
                //     all_commit_characters: None,
                //     ..Default::default()
                // }),
                // execute_command_provider: Some(ExecuteCommandOptions {
                //     commands: vec!["dummy.do_something".to_string()],
                //     work_done_progress_options: Default::default(),
                // }),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            work_done_progress_options: Default::default(),
                            legend: lsp_legend(),
                            range: Some(false),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                        },
                    ),
                ),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_change_workspace_folders(&self, _: DidChangeWorkspaceFoldersParams) {
        self.client
            .log_message(MessageType::INFO, "workspace folders changed!")
            .await;
    }

    async fn did_change_configuration(&self, _: DidChangeConfigurationParams) {
        self.client
            .log_message(MessageType::INFO, "configuration changed!")
            .await;
    }

    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {
        self.client
            .log_message(MessageType::INFO, "watched files have changed!")
            .await;
    }

    // async fn execute_command(&self, _: ExecuteCommandParams) -> Result<Option<Value>> {
    //     self.client
    //         .log_message(MessageType::INFO, "command executed!")
    //         .await;

    //     match self.client.apply_edit(WorkspaceEdit::default()).await {
    //         Ok(res) if res.applied => self.client.log_message(MessageType::INFO, "applied").await,
    //         Ok(_) => self.client.log_message(MessageType::INFO, "rejected").await,
    //         Err(err) => self.client.log_message(MessageType::ERROR, err).await,
    //     }

    //     Ok(None)
    // }

    async fn did_open(&self, _: DidOpenTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file opened!")
            .await;
    }

    async fn did_change(&self, _: DidChangeTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file changed!")
            .await;
    }

    async fn did_save(&self, _: DidSaveTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file saved!")
            .await;
    }

    async fn did_close(&self, _: DidCloseTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file closed!")
            .await;
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let Some(document) = params.text_document.uri.to_file_path() else {
            return Ok(None);
        };

        self.client
            .log_message(
                MessageType::INFO,
                format!("semantic tokens full: {document:?}"),
            )
            .await;

        let Ok(document) = std::fs::read_to_string(document) else {
            return Ok(None);
        };

        let highlights = highlights::highlights(&document);
        self.client
            .log_message(MessageType::INFO, format!("highlights: {highlights:#?}"))
            .await;

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            data: highlights,
            ..Default::default()
        })))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        self.client
            .log_message(MessageType::INFO, "On Hover".to_string())
            .await;

        let selected = params.text_document_position_params.position;
        let selected = pos_to_point(selected);

        let Some(document) = params
            .text_document_position_params
            .text_document
            .uri
            .to_file_path()
        else {
            self.client
                .log_message(
                    MessageType::WARNING,
                    "On Hover: Could not get file path".to_string(),
                )
                .await;
            return Ok(None);
        };

        let filename = document.display().to_string();
        let Ok(document) = std::fs::read_to_string(document) else {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!("On Hover: Could not get file content: {filename}"),
                )
                .await;
            return Ok(None);
        };

        let (sources, source_id) = Sources::single(&filename, &document);
        let mut asts = ASTS::new();
        let Ok(ast) = asts.parse(source_id, sources.get(source_id)) else {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!("On Hover: Could not parse file: {filename}"),
                )
                .await;
            return Ok(None);
        };
        let Some(root) = ast.root_id() else {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!("On Hover: Could not get parsed root SEXP: {filename}"),
                )
                .await;
            return Ok(None);
        };
        let Some(selected) = ast.get_by_point(selected) else {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!("On Hover: Could not get selected SEXP: {filename}"),
                )
                .await;
            return Ok(None);
        };
        // let mut env = TypeEnv::default().with_prelude(&mut sources);
        let type_ = {
            let mut modules: FileModules = sources.into();
            // let sources: &mut Sources = &mut sources;
            let asts: &mut ASTS = &mut asts;
            let prelude = prelude();
            let envs = &[prelude];
            let (root, mut diagnostics) = process_ast(asts, root, envs);
            let mut type_env = TypeEnv::new().with_prelude(modules.sources_mut());
            type_env.type_term(asts, root, &mut diagnostics, &mut modules, 0);
            // let type_ = type_env.check(asts, selected, &mut diagnostics);
            let Some(infered) = type_env.get_infered(selected) else {
                return Ok(None);
            };

            let type_ = type_env.coalesce(infered);
            // for diag in diagnostics.print(sources) {}

            let ty_ = type_env.to_string(type_);
            if !diagnostics.has_errors() {
                ty_
            } else {
                let errs = diagnostics.pretty_print(modules.sources());
                format!("{ty_}\n# Errors:\n```\n{errs}\n```")
            }
        };
        self.client
            .log_message(MessageType::INFO, format!("On Hover infered type: {type_}"))
            .await;

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: type_,
            }),
            range: None,
        }))
    }

    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> Result<DocumentDiagnosticReportResult> {
        let items = self.diagnostics_inner(params).await.unwrap_or_default();

        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items,
                },
            }),
        ))
    }

    // async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
    //     Ok(Some(CompletionResponse::Array(vec![
    //         CompletionItem::new_simple("Hello".to_string(), "Some detail".to_string()),
    //         CompletionItem::new_simple("Bye".to_string(), "More detail".to_string()),
    //     ])))
    // }
}

impl Backend {
    async fn diagnostics_inner(&self, params: DocumentDiagnosticParams) -> Option<Vec<Diagnostic>> {
        self.client
            .log_message(MessageType::INFO, format!("On Diagnostic: {params:?}"))
            .await;

        let Some(document) = params.text_document.uri.to_file_path() else {
            self.client
                .log_message(
                    MessageType::WARNING,
                    "On Diag: Could not get file path".to_string(),
                )
                .await;
            return None;
        };

        let filename = document.display().to_string();
        let Ok(document) = std::fs::read_to_string(document) else {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!("On Diag: Could not get file content: {filename}"),
                )
                .await;
            return None;
        };

        let (sources, source_id) = Sources::single(&filename, &document);
        let mut asts = ASTS::new();
        let Ok(ast) = asts.parse(source_id, sources.get(source_id)) else {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!("On Diag: Could not parse file: {filename}"),
                )
                .await;
            return None;
        };
        let Some(root) = ast.root_id() else {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!("On Diag: Could not get parsed root SEXP: {filename}"),
                )
                .await;
            return None;
        };

        let (modules, diag) = {
            let mut modules: FileModules = sources.into();
            let asts: &mut ASTS = &mut asts;
            let prelude = prelude();
            let envs = &[prelude];
            let (root, mut diagnostics) = process_ast(asts, root, envs);
            let mut type_env = TypeEnv::new().with_prelude(modules.sources_mut());
            type_env.type_term(asts, root, &mut diagnostics, &mut modules, 0);
            (modules, diagnostics)
        };
        let diag = diag
            .into_iter()
            .map(|d| from_diag(d, modules.sources(), source_id))
            .collect();
        self.client
            .log_message(MessageType::INFO, format!("On Diag: {diag:#?}"))
            .await;
        Some(diag)
    }
}

fn from_diag(
    value: crate::diagnostics::Diag,
    sources: &Sources,
    current_file: SourceId,
) -> Diagnostic {
    let mut range = value.span.range;
    let mut message = value.message.clone();
    let mut missing_main = false;
    if current_file != value.span.source_id {
        for extra in value.extras.iter() {
            let Some(span) = extra.span else {
                continue;
            };
            if span.source_id == current_file && !missing_main {
                range = span.range;
                missing_main = true;
            }
            if span.source_id != current_file {
                message += &format!("\n{}", extra.message);
            }
        }
    }
    let range = ts_range_to_range(range);
    let related = value
        .extras
        .into_iter()
        .filter_map(|e| from_extra(e, sources))
        .collect();
    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        code_description: None,
        source: Some("project-s".to_string()),
        message,
        related_information: Some(related),
        tags: None,
        data: None,
    }
}

fn from_extra(
    value: crate::diagnostics::Extra,
    sources: &Sources,
) -> Option<DiagnosticRelatedInformation> {
    let span = value.span.unwrap();
    let range = ts_range_to_range(span.range);
    let uri = span.source_id;
    let source = sources.get(uri);
    let uri = format!("file://{}", source.filename);
    let location = Location {
        uri: Uri::from_str(&uri).ok()?,
        range,
    };
    Some(DiagnosticRelatedInformation {
        location,
        message: value.message,
    })
}
