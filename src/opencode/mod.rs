pub mod event;
pub mod runner;
pub mod session;

use crate::config::{normalize_model, Config};
use crate::error::{OcError, Result};

pub use crate::config::Command;

/// Resolve the model for a command invocation.
///
/// Priority:
/// 1. `-m/--model` given on the command line;
/// 2. `commands.<name>` in `~/.config/oclikits/config.jsonc`;
/// 3. the global `model` in the same file;
/// 4. [`OcError::ModelNotConfigured`], which points at the config template.
pub fn resolve_model(command: Command, cli_model: &Option<String>) -> Result<String> {
    if let Some(model) = normalize_model(cli_model) {
        return Ok(model);
    }

    Config::load()?
        .resolve(command)
        .ok_or(OcError::ModelNotConfigured)
}
