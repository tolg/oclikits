pub mod sse;
pub mod types;

use crate::config::Config;
use crate::error::{AidError, Result};
use reqwest::Client;
use types::{ApiErrorResponse, ChatCompletionRequest, ChatCompletionResponse, ChatMessage};

pub struct ApiClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl ApiClient {
    pub fn new(cfg: &Config) -> Result<Self> {
        let api_key = cfg.api_key()?;
        let base_url = cfg.base_url();
        let user_agent = format!("aiclik/{} ({}; {})", env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH);
        let client = Client::builder()
            .user_agent(user_agent)
            .build()?;

        Ok(Self {
            client,
            api_key,
            base_url,
        })
    }

    pub async fn chat(
        &self,
        model: &str,
        system_prompt: &str,
        user_prompt: &str,
        temperature: f64,
    ) -> Result<String> {
        let messages = vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(user_prompt),
        ];

        let payload = ChatCompletionRequest {
            model: model.to_string(),
            messages,
            temperature,
            stream: Some(false),
        };

        let url = format!("{}/chat/completions", self.base_url);
        let res = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let status = res.status();
        let text = res.text().await?;

        if !status.is_success() {
            let message = serde_json::from_str::<ApiErrorResponse>(&text)
                .ok()
                .and_then(|e| e.error)
                .map(|e| e.message)
                .unwrap_or(text);

            return Err(AidError::Api {
                status: status.as_u16(),
                message,
            });
        }

        let parsed: ChatCompletionResponse = serde_json::from_str(&text)?;
        if let Some(err) = parsed.error {
            return Err(AidError::Api {
                status: status.as_u16(),
                message: err.message,
            });
        }

        let content = parsed
            .choices
            .first()
            .and_then(|c| c.message.as_ref())
            .map(|m| m.content.clone())
            .ok_or(AidError::EmptyResponse)?;

        Ok(content)
    }

    pub async fn chat_messages_stream(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        temperature: f64,
        on_chunk: impl FnMut(&str),
    ) -> Result<String> {
        let payload = ChatCompletionRequest {
            model: model.to_string(),
            messages,
            temperature,
            stream: Some(true),
        };

        let url = format!("{}/chat/completions", self.base_url);
        let res = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        sse::stream_response(res, on_chunk).await
    }
}
