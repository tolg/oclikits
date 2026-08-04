use std::fmt;

#[derive(Debug)]
pub enum AidError {
    Config(String),
    Network(reqwest::Error),
    Api { status: u16, message: String },
    EmptyResponse,
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::error::Error for AidError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AidError::Network(e) => Some(e),
            AidError::Io(e) => Some(e),
            AidError::Json(e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for AidError {
    fn from(e: reqwest::Error) -> Self {
        AidError::Network(e)
    }
}

impl From<std::io::Error> for AidError {
    fn from(e: std::io::Error) -> Self {
        AidError::Io(e)
    }
}

impl From<serde_json::Error> for AidError {
    fn from(e: serde_json::Error) -> Self {
        AidError::Json(e)
    }
}

impl fmt::Display for AidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if crate::i18n::is_zh() {
            match self {
                AidError::Config(msg) => write!(f, "配置错误: {}", msg),
                AidError::Network(err) => write!(f, "网络请求错误: {}", err),
                AidError::Api { status, message } => {
                    write!(f, "API 调用失败 (HTTP {}): {}", status, message)
                }
                AidError::EmptyResponse => write!(f, "API 返回解析为空"),
                AidError::Io(err) => write!(f, "IO 错误: {}", err),
                AidError::Json(err) => write!(f, "JSON 解析错误: {}", err),
            }
        } else {
            match self {
                AidError::Config(msg) => write!(f, "Config error: {}", msg),
                AidError::Network(err) => write!(f, "Network error: {}", err),
                AidError::Api { status, message } => {
                    write!(f, "API call failed (HTTP {}): {}", status, message)
                }
                AidError::EmptyResponse => write!(f, "API response is empty"),
                AidError::Io(err) => write!(f, "IO error: {}", err),
                AidError::Json(err) => write!(f, "JSON parse error: {}", err),
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, AidError>;
