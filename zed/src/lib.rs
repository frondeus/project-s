use zed_extension_api::Result;
use zed_extension_api::{self as zed, LanguageServerId};

struct MyExtension;

impl zed::Extension for MyExtension {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self
    }
    // ...
    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        Ok(zed::Command {
            command: "/Users/frondeus/Code/project-s/target/debug/project-s".into(),
            args: vec![],
            env: Default::default(),
            // command: get_path_to_language_server_executable()?,
            // args: get_args_for_language_server()?,
            // env: get_env_for_language_server()?,
        })
    }
}

zed::register_extension!(MyExtension);
