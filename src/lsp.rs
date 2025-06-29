// use std::{
//     path::{Path, PathBuf},
//     sync::Arc,
// };

use highlights::lsp_legend;
// use tokio::sync::Mutex;
use tower_lsp_server::{
    Client, LanguageServer, UriExt,
    jsonrpc::Result,
    lsp_types::{
        DidChangeConfigurationParams, DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
        DidChangeWorkspaceFoldersParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
        DidSaveTextDocumentParams, Hover, HoverContents, HoverParams, HoverProviderCapability,
        InitializeParams, InitializeResult, InitializedParams, MarkupContent, MarkupKind,
        MessageType, OneOf, Position, SemanticTokens, SemanticTokensFullOptions,
        SemanticTokensOptions, SemanticTokensParams, SemanticTokensResult,
        SemanticTokensServerCapabilities, ServerCapabilities, TextDocumentSyncCapability,
        TextDocumentSyncKind, WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
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

use crate::{
    ast::{ASTS, SExpParser},
    process_ast,
    s_std::prelude,
    source::Sources,
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
                format!("semantic tokens full: {:?}", document),
            )
            .await;

        let Ok(document) = std::fs::read_to_string(document) else {
            return Ok(None);
        };

        let highlights = highlights::highlights(&document);
        self.client
            .log_message(MessageType::INFO, format!("highlights: {:#?}", highlights))
            .await;

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            data: highlights,
            ..Default::default()
        })))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        self.client
            .log_message(MessageType::INFO, format!("On Hover"))
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
                    format!("On Hover: Could not get file path"),
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

        let (mut sources, source_id) = Sources::single(&filename, &document);
        let mut asts = ASTS::new();
        let Ok(tree) = SExpParser::parse_tree(&document) else {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!("Could not parse file: {filename}"),
                )
                .await;
            return Ok(None);
        };
        let Some(selected) = tree
            .root_node()
            .named_descendant_for_point_range(selected, selected)
        else {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!("On Hover: Could not find selected node: {filename}"),
                )
                .await;
            return Ok(None);
        };
        let Ok(ast) = asts.parse_with_tree(tree, source_id, sources.get(source_id)) else {
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
        let prelude = prelude();
        // let mut env = TypeEnv::default().with_prelude(&mut sources);
        let type_ = {
            let sources: &mut Sources = &mut sources;
            let asts: &mut ASTS = &mut asts;
            let envs = &[prelude];
            let (_root, mut diagnostics) = process_ast(asts, root, envs);
            let mut type_env = TypeEnv::default().with_prelude(sources);
            let type_ = type_env.check(asts, root, &mut diagnostics);
            type_env.to_string(type_)
        };
        self.client
            .log_message(MessageType::INFO, format!("On Hover: {type_}"))
            .await;

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::PlainText,
                value: type_,
            }),
            range: None,
        }))
    }

    // async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
    //     Ok(Some(CompletionResponse::Array(vec![
    //         CompletionItem::new_simple("Hello".to_string(), "Some detail".to_string()),
    //         CompletionItem::new_simple("Bye".to_string(), "More detail".to_string()),
    //     ])))
    // }
}
