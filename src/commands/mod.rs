pub mod commit;
pub mod q;
pub mod shell;
pub mod sq;
pub mod trans;

use crate::error::Result;
use crate::opencode::session::delete_opencode_session;
use crate::ui;

/// Delete an OpenCode session.
///
/// `strict` controls how a deletion failure is handled:
/// `true` propagates the error (`--clear`), `false` only warns (`--new`),
/// because starting a fresh session is still useful in that case.
pub async fn delete_session(session_id: &str, strict: bool) -> Result<()> {
    match delete_opencode_session(session_id).await {
        Ok(()) => Ok(()),
        Err(err) if strict => Err(err),
        Err(err) => {
            ui::warn(&err.to_string());
            Ok(())
        }
    }
}
