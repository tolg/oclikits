use serde::Deserialize;

/// One NDJSON event emitted by `opencode run --format json`.
///
/// Observed shapes:
///
/// ```json
/// {"type":"step_start","sessionID":"ses_xxx","part":{...}}
/// {"type":"text","sessionID":"ses_xxx","part":{"type":"text","text":"hello"}}
/// {"type":"error","sessionID":"","error":{"type":"unknown","message":"Session not found"}}
/// ```
#[derive(Debug, Deserialize)]
pub struct OpenCodeEvent {
    #[serde(rename = "type", default)]
    pub event_type: String,
    #[serde(rename = "sessionID", default)]
    pub session_id: String,
    #[serde(default)]
    pub part: Option<EventPart>,
    #[serde(default)]
    pub error: Option<EventError>,
}

impl OpenCodeEvent {
    pub fn is_text(&self) -> bool {
        self.event_type == "text" && self.part_type() == Some("text") && self.text().is_some()
    }

    pub fn is_error(&self) -> bool {
        self.event_type == "error"
    }

    pub fn is_tool_use(&self) -> bool {
        self.event_type == "tool_use"
    }

    pub fn text(&self) -> Option<&str> {
        self.part.as_ref().and_then(|part| part.text.as_deref())
    }

    pub fn error_message(&self) -> Option<&str> {
        self.error
            .as_ref()
            .and_then(|error| error.message.as_deref())
    }

    fn part_type(&self) -> Option<&str> {
        self.part.as_ref().map(|part| part.part_type.as_str())
    }
}

#[derive(Debug, Deserialize)]
pub struct EventPart {
    #[serde(rename = "type", default)]
    pub part_type: String,
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EventError {
    #[serde(rename = "type", default)]
    #[allow(dead_code)]
    pub error_type: String,
    #[serde(default)]
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_text_event() {
        let line = r#"{"type":"text","timestamp":1,"sessionID":"ses_1","part":{"id":"prt_1","type":"text","text":"hello"}}"#;
        let event: OpenCodeEvent = serde_json::from_str(line).unwrap();
        assert_eq!(event.session_id, "ses_1");
        assert!(event.is_text());
        assert_eq!(event.text(), Some("hello"));
    }

    #[test]
    fn parses_step_event_without_text() {
        let line = r#"{"type":"step_start","sessionID":"ses_1","part":{"type":"step-start"}}"#;
        let event: OpenCodeEvent = serde_json::from_str(line).unwrap();
        assert!(!event.is_text());
        assert!(!event.is_error());
        assert_eq!(event.text(), None);
    }

    #[test]
    fn parses_error_event() {
        let line = r#"{"type":"error","sessionID":"","error":{"type":"unknown","message":"Session not found"}}"#;
        let event: OpenCodeEvent = serde_json::from_str(line).unwrap();
        assert!(event.is_error());
        assert_eq!(event.error_message(), Some("Session not found"));
    }
}
