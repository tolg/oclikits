use thiserror::Error;

#[derive(Error, Debug)]
pub enum AidError {
    #[error("配置错误: {0}")]
    Config(String),

    #[error("网络请求错误: {0}")]
    Network(#[from] reqwest::Error),

    #[error("API 调用失败 (HTTP {status}): {message}")]
    Api { status: u16, message: String },

    #[error("API 返回解析为空")]
    EmptyResponse,

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON 解析错误: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, AidError>;
