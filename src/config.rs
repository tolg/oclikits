use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{OcError, Result};
use crate::i18n::tr;

/// oclikits configuration directory.
///
/// Follows `XDG_CONFIG_HOME` when set, otherwise `~/.config/oclikits`.
pub fn config_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.trim().is_empty() {
            return PathBuf::from(xdg).join("oclikits");
        }
    }

    if let Some(home) = dirs::home_dir() {
        return home.join(".config").join("oclikits");
    }

    dirs::config_dir()
        .map(|dir| dir.join("oclikits"))
        .unwrap_or_else(|| PathBuf::from(".oclikits"))
}

/// Path of the configuration file, `~/.config/oclikits/config.jsonc`.
pub fn config_path() -> PathBuf {
    config_dir().join("config.jsonc")
}

/// Legacy configuration path, used before the file was renamed to `.jsonc`.
fn legacy_config_path() -> PathBuf {
    config_dir().join("config.json")
}

/// Commented template written to `config.jsonc` when it does not exist yet.
///
/// It uses JSON with `//` comments (JSONC). Comments are stripped before
/// parsing, so this file stays valid input for [`Config::load`].
pub const CONFIG_TEMPLATE: &str = r#"{
  // oclikits 模型配置，值均为 OpenCode 的 provider/model 格式。
  // 支持 // 与 /* */ 注释，解析时会自动忽略。
  // 也可以用 -m/--model 在单次执行时临时覆盖这里的配置。

  // 全局默认模型：所有命令在未配置专用模型时使用它。
  // 可留空或不配置；此时不向 opencode 传 --model，改用 OpenCode 自身的默认模型
  // （OpenCode 全局/项目配置，或会话已选模型）。
  "model": "",

  // 各命令的专用模型（可选）。留空或删除某项时，回退到上面的 "model"。
  "commands": {
    "q": "",      // ocq：连续快速问答
    "sq": "",     // ocsq：单次问答（无历史）
    "trans": "",  // oct / octrans：中英翻译、错别字纠正
    "shell": "",  // ocsh：绑定当前目录的 Shell Agent
    "commit": ""  // occmt：Git 提交
  }
}
"#;

/// Logical command identity, used to look up a command-specific model.
///
/// The matching key under `commands` in `config.jsonc` is the lower-case name:
/// `q`, `sq`, `trans`, `shell`, `commit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Q,
    Sq,
    Trans,
    Shell,
    Commit,
}

/// Contents of `~/.config/oclikits/config.jsonc`.
#[derive(Debug, Default, Deserialize)]
pub struct Config {
    /// Global default model in `provider/model` format.
    #[serde(default)]
    pub model: Option<String>,
    /// Optional per-command model overrides.
    #[serde(default)]
    pub commands: CommandModels,
}

/// Optional per-command models, keyed by [`Command::as_str`].
#[derive(Debug, Default, Deserialize)]
pub struct CommandModels {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub sq: Option<String>,
    #[serde(default)]
    pub trans: Option<String>,
    #[serde(default)]
    pub shell: Option<String>,
    #[serde(default)]
    pub commit: Option<String>,
}

impl CommandModels {
    fn get(&self, command: Command) -> Option<String> {
        let value = match command {
            Command::Q => &self.q,
            Command::Sq => &self.sq,
            Command::Trans => &self.trans,
            Command::Shell => &self.shell,
            Command::Commit => &self.commit,
        };
        normalize_model(value)
    }
}

impl Config {
    /// Load the configuration file.
    ///
    /// When the file does not exist, a commented template is written to disk
    /// (best effort) and an empty config is returned, so the caller can report
    /// a precise "not configured" error pointing at the template. A legacy
    /// `config.json` is migrated to `config.jsonc` first.
    pub fn load() -> Result<Self> {
        let path = config_path();

        // Best effort: a read-only config dir still yields a precise
        // "not configured" error below.
        let _ = write_config_template();

        match fs::read_to_string(&path) {
            Ok(content) => Self::parse(&content).map_err(|err| {
                let zh = format!("配置文件解析失败：{}\n{}", path.display(), err);
                let en = format!("Failed to parse config file {}: {}", path.display(), err);
                OcError::Message(tr(&zh, &en).to_string())
            }),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(err.into()),
        }
    }

    /// Parse JSONC config content (JSON plus `//` and `/* */` comments).
    pub fn parse(content: &str) -> serde_json::Result<Self> {
        serde_json::from_str(&strip_json_comments(content))
    }

    /// The global default model, trimmed. `None` when unset or blank.
    pub fn model(&self) -> Option<String> {
        normalize_model(&self.model)
    }

    /// The effective model for `command`, ignoring the command line.
    ///
    /// Falls back from the command-specific model to the global default.
    pub fn resolve(&self, command: Command) -> Option<String> {
        self.commands.get(command).or_else(|| self.model())
    }
}

/// Trim a model string and treat blank values as unset.
pub fn normalize_model(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

/// Create the config file from the commented template when it is missing.
///
/// - An existing `config.jsonc` is never overwritten.
/// - A legacy `config.json` is migrated (renamed) to `config.jsonc`.
///
/// Returns `true` when a fresh template was written. Used by `oc init-config`
/// at install time and as a first-run fallback.
pub fn write_config_template() -> Result<bool> {
    ensure_config_file(&config_path(), &legacy_config_path())
}

/// Core of [`write_config_template`], parameterized by path for testing.
fn ensure_config_file(config: &Path, legacy: &Path) -> Result<bool> {
    if config.exists() {
        return Ok(false);
    }

    if legacy.exists() {
        fs::rename(legacy, config)?;
        return Ok(false);
    }

    if let Some(parent) = config.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(config, CONFIG_TEMPLATE)?;
    Ok(true)
}

/// Remove `//` line comments and `/* */` block comments from JSONC input.
///
/// Comment markers inside strings are preserved, and newlines are kept so that
/// serde error positions remain meaningful.
fn strip_json_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }

        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            '/' => match chars.peek() {
                Some('/') => {
                    // Line comment: drop it but keep the trailing newline.
                    chars.next();
                    for nc in chars.by_ref() {
                        if nc == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                }
                Some('*') => {
                    // Block comment: drop it, keeping newlines for line numbers.
                    chars.next();
                    let mut prev = '\0';
                    for nc in chars.by_ref() {
                        if nc == '\n' {
                            out.push('\n');
                        }
                        if prev == '*' && nc == '/' {
                            break;
                        }
                        prev = nc;
                    }
                }
                _ => out.push(c),
            },
            _ => out.push(c),
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_is_none_when_unset_or_blank() {
        let unset: Config = serde_json::from_str("{}").unwrap();
        assert_eq!(unset.model(), None);

        let blank: Config = serde_json::from_str(r#"{"model": "   "}"#).unwrap();
        assert_eq!(blank.model(), None);
    }

    #[test]
    fn model_is_trimmed_when_set() {
        let config: Config = serde_json::from_str(r#"{"model": "  opencode-go/x "}"#).unwrap();
        assert_eq!(config.model().as_deref(), Some("opencode-go/x"));
    }

    #[test]
    fn strips_line_and_block_comments() {
        let json = "{\n  // line\n  \"model\": \"a/b\" /* block */\n}";
        let config = Config::parse(json).unwrap();
        assert_eq!(config.model().as_deref(), Some("a/b"));
    }

    #[test]
    fn keeps_comment_markers_inside_strings() {
        let json = r#"{"model": "https://example.com/a//b/*c*/d"}"#;
        let config = Config::parse(json).unwrap();
        assert_eq!(
            config.model().as_deref(),
            Some("https://example.com/a//b/*c*/d")
        );
    }

    #[test]
    fn template_is_valid_jsonc_and_empty() {
        let config = Config::parse(CONFIG_TEMPLATE).unwrap();
        assert_eq!(config.model(), None);
        assert_eq!(config.resolve(Command::Q), None);
    }

    #[test]
    fn command_model_overrides_global_default() {
        let config =
            Config::parse(r#"{"model": "g/global", "commands": {"q": "q/model", "shell": "   "}}"#)
                .unwrap();

        assert_eq!(config.resolve(Command::Q).as_deref(), Some("q/model"));
        // Blank command value falls back to the global default.
        assert_eq!(config.resolve(Command::Shell).as_deref(), Some("g/global"));
        // Unset command value falls back to the global default.
        assert_eq!(config.resolve(Command::Sq).as_deref(), Some("g/global"));
    }

    #[test]
    fn ensure_config_file_creates_once_and_is_valid_jsonc() {
        let dir = std::env::temp_dir().join(format!(
            "oclikits-config-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let config = dir.join("config.jsonc");
        let legacy = dir.join("config.json");
        let _ = fs::remove_dir_all(&dir);

        assert!(ensure_config_file(&config, &legacy).unwrap());
        // Never overwrite an existing file.
        assert!(!ensure_config_file(&config, &legacy).unwrap());

        let parsed = Config::parse(&fs::read_to_string(&config).unwrap()).unwrap();
        assert_eq!(parsed.model(), None);
        assert_eq!(parsed.resolve(Command::Commit), None);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn ensure_config_file_migrates_legacy_json() {
        let dir = std::env::temp_dir().join(format!(
            "oclikits-config-migrate-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let config = dir.join("config.jsonc");
        let legacy = dir.join("config.json");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(&legacy, "{\"model\": \"legacy/model\"}\n").unwrap();

        // Migration is not "creating a fresh template".
        assert!(!ensure_config_file(&config, &legacy).unwrap());
        assert!(!legacy.exists());
        assert_eq!(
            Config::parse(&fs::read_to_string(&config).unwrap())
                .unwrap()
                .model()
                .as_deref(),
            Some("legacy/model")
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
