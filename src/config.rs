use crate::error::{AidError, Result};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default, Clone)]
pub struct OpenAiConfig {
    #[serde(rename = "apiKey")]
    pub api_key: Option<String>,
    #[serde(rename = "baseUrl")]
    pub base_url: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ModelsConfig {
    pub default: Option<String>,
    pub shell: Option<String>,
    pub ask: Option<String>,
    pub trans: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct DefaultsConfig {
    pub temperature: Option<f64>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Config {
    pub openai: Option<OpenAiConfig>,
    pub models: Option<ModelsConfig>,
    pub defaults: Option<DefaultsConfig>,
}

impl Config {
    pub fn config_dir() -> Option<PathBuf> {
        if let Some(home) = dirs::home_dir() {
            return Some(home.join(".config").join("aiclikits"));
        }
        dirs::config_dir().map(|p| p.join("aiclikits"))
    }

    pub fn config_file_path() -> Option<PathBuf> {
        Self::config_dir().map(|p| p.join("config.json"))
    }

    pub fn load() -> Self {
        let mut paths = Vec::new();
        if let Some(p) = Self::config_file_path() {
            paths.push(p);
        }
        if let Some(sys_cfg) = dirs::config_dir() {
            paths.push(sys_cfg.join("aiclikits").join("config.json"));
        }

        for path in paths {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(cfg) = serde_json::from_str::<Config>(&content) {
                        return cfg;
                    }
                }
            }
        }
        Config::default()
    }

    pub fn api_key(&self) -> Result<String> {
        if let Ok(env_key) = std::env::var("OPENAI_API_KEY") {
            let trimmed = env_key.trim().to_string();
            if !trimmed.is_empty() {
                return Ok(trimmed);
            }
        }

        if let Some(ref openai) = self.openai {
            if let Some(ref key) = openai.api_key {
                let trimmed = key.trim().to_string();
                if !trimmed.is_empty() {
                    return Ok(trimmed);
                }
            }
        }

        let path_str = Self::config_file_path()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "~/.config/aiclikits/config.json".to_string());
        if crate::i18n::is_zh() {
            Err(AidError::Config(format!(
                "未在配置文件设置 openai.apiKey ({}) 或环境变量 OPENAI_API_KEY",
                path_str
            )))
        } else {
            Err(AidError::Config(format!(
                "openai.apiKey is not set in config file ({}) nor OPENAI_API_KEY environment variable",
                path_str
            )))
        }
    }

    pub fn base_url(&self) -> String {
        if let Ok(env_url) = std::env::var("OPENAI_BASE_URL") {
            let trimmed = env_url.trim().trim_end_matches('/').to_string();
            if !trimmed.is_empty() {
                return trimmed;
            }
        }

        let base = self
            .openai
            .as_ref()
            .and_then(|o| o.base_url.as_ref())
            .map(|s| s.trim().trim_end_matches('/').to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        base
    }

    pub fn model_for(&self, tool: &str, fallback: &str) -> String {
        let models = self.models.as_ref();
        let specific = match tool {
            "shell" => models.and_then(|m| m.shell.as_ref()),
            "ask" | "q" => models.and_then(|m| m.ask.as_ref()),
            "trans" => models.and_then(|m| m.trans.as_ref()),
            _ => None,
        };

        if let Some(model) = specific {
            let trimmed = model.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }

        if let Some(default_model) = models.and_then(|m| m.default.as_ref()) {
            let trimmed = default_model.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }

        fallback.to_string()
    }

    pub fn temperature(&self) -> f64 {
        self.defaults
            .as_ref()
            .and_then(|d| d.temperature)
            .unwrap_or(0.1)
    }
}
