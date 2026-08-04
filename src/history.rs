use crate::client::types::ChatMessage;
use serde_json;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

pub struct HistoryManager {
    file_path: PathBuf,
    max_history: usize,
    ttl: Duration,
}

impl HistoryManager {
    pub fn new() -> Self {
        let file_path = crate::config::Config::config_dir()
            .map(|p| p.join("aiq_context.json"))
            .unwrap_or_else(|| PathBuf::from("aiq_context.json"));

        Self {
            file_path,
            max_history: 100,
            ttl: Duration::from_secs(100_800), // 28 hours
        }
    }

    pub fn clear(&self) {
        if self.file_path.exists() {
            let _ = fs::remove_file(&self.file_path);
        }
    }

    pub fn load(&self) -> Vec<ChatMessage> {
        if !self.file_path.exists() {
            return Vec::new();
        }

        if let Ok(metadata) = fs::metadata(&self.file_path) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                    if elapsed > self.ttl {
                        let _ = fs::remove_file(&self.file_path);
                        return Vec::new();
                    }
                }
            }
        }

        if let Ok(content) = fs::read_to_string(&self.file_path) {
            if let Ok(history) = serde_json::from_str::<Vec<ChatMessage>>(&content) {
                return history;
            }
        }

        Vec::new()
    }

    pub fn save(&self, mut history: Vec<ChatMessage>) {
        if history.len() > self.max_history {
            let start = history.len() - self.max_history;
            history = history.split_off(start);
        }

        if let Some(parent) = self.file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if let Ok(content) = serde_json::to_string(&history) {
            let _ = fs::write(&self.file_path, content);
        }
    }
}
