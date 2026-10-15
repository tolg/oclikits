pub mod event;
pub mod runner;
pub mod session;

use crate::config::{normalize_model, Config};
use crate::error::Result;

pub use crate::config::Command;

/// Resolve the model for a command invocation, if one is configured.
///
/// Priority:
/// 1. `-m/--model` given on the command line;
/// 2. `commands.<name>` in `~/.config/oclikits/config.jsonc`;
/// 3. the global `model` in the same file.
///
/// Returns `None` when none of them is set. The caller then omits `--model`,
/// so `opencode run` falls back to its own default model (OpenCode's global or
/// project configuration, or the session's already-selected model).
pub fn resolve_model(command: Command, cli_model: &Option<String>) -> Result<Option<String>> {
    if let Some(model) = normalize_model(cli_model) {
        return Ok(Some(model));
    }

    Ok(Config::load()?.resolve(command))
}
