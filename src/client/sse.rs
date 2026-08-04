use crate::client::types::ChatCompletionResponse;
use crate::error::{AidError, Result};
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use reqwest::Response;
use std::io::{self, Write};

pub async fn stream_response(
    response: Response,
    mut on_chunk: impl FnMut(&str),
) -> Result<String> {
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        let message = serde_json::from_str::<crate::client::types::ApiErrorResponse>(&text)
            .ok()
            .and_then(|e| e.error)
            .map(|e| e.message)
            .unwrap_or(text);

        return Err(AidError::Api {
            status: status.as_u16(),
            message,
        });
    }

    let mut full_text = String::new();
    let mut stream = response.bytes_stream().eventsource();

    while let Some(event_result) = stream.next().await {
        match event_result {
            Ok(event) => {
                let data = event.data.trim();
                if data == "[DONE]" {
                    break;
                }
                if let Ok(parsed) = serde_json::from_str::<ChatCompletionResponse>(data) {
                    if let Some(err) = parsed.error {
                        return Err(AidError::Api {
                            status: 200,
                            message: err.message,
                        });
                    }
                    if let Some(choice) = parsed.choices.first() {
                        if let Some(delta) = &choice.delta {
                            if let Some(content) = &delta.content {
                                on_chunk(content);
                                full_text.push_str(content);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                return Err(AidError::Config(format!("SSE stream error: {}", e)));
            }
        }
    }

    io::stdout().flush().ok();
    Ok(full_text)
}
