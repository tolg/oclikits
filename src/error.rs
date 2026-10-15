use std::fmt;

use crate::i18n::tr;

#[derive(Debug)]
pub enum OcError {
    /// Generic user-facing error message.
    Message(String),
    /// The `opencode` executable could not be found.
    OpencodeNotFound,
    /// The OpenCode session referenced by the local mapping no longer exists.
    SessionNotFound(String),
    /// The command was interrupted (SIGINT / SIGTERM).
    Interrupted,
    /// An `opencode` process finished with a non-zero exit code.
    ExitCode(i32),
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::error::Error for OcError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OcError::Io(err) => Some(err),
            OcError::Json(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for OcError {
    fn from(err: std::io::Error) -> Self {
        OcError::Io(err)
    }
}

impl From<serde_json::Error> for OcError {
    fn from(err: serde_json::Error) -> Self {
        OcError::Json(err)
    }
}

impl fmt::Display for OcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OcError::Message(msg) => write!(f, "{}", msg),
            OcError::OpencodeNotFound => write!(
                f,
                "{}",
                tr(
                    "未找到 opencode 命令，请先安装 OpenCode 并确保它在 PATH 中。",
                    "opencode command not found. Install OpenCode and make sure it is in PATH."
                )
            ),
            OcError::SessionNotFound(id) => {
                let zh = format!("OpenCode 会话不存在：{}", id);
                let en = format!("OpenCode session not found: {}", id);
                write!(f, "{}", tr(&zh, &en))
            }
            OcError::Interrupted => write!(f, "{}", tr("已中断。", "Interrupted.")),
            OcError::ExitCode(code) => {
                let zh = format!("opencode 退出码：{}", code);
                let en = format!("opencode exited with code {}", code);
                write!(f, "{}", tr(&zh, &en))
            }
            OcError::Io(err) => {
                let zh = format!("IO 错误：{}", err);
                let en = format!("IO error: {}", err);
                write!(f, "{}", tr(&zh, &en))
            }
            OcError::Json(err) => {
                let zh = format!("JSON 解析错误：{}", err);
                let en = format!("JSON parse error: {}", err);
                write!(f, "{}", tr(&zh, &en))
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, OcError>;
