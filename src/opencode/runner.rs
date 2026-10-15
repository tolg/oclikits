use std::io;
use std::path::PathBuf;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::error::{OcError, Result};
use crate::i18n::tr;
use crate::opencode::event::OpenCodeEvent;
use crate::opencode::session::delete_opencode_session;
use crate::ui::{self, preview::Preview};

pub struct RunOptions {
    /// Explicit model in `provider/model` format. `None` lets `opencode run`
    /// use its own default model instead of passing `--model`.
    pub model: Option<String>,
    pub session_id: Option<String>,
    pub directory: Option<PathBuf>,
    pub prompt: String,
    /// Delete the OpenCode session when this run finishes, on success, failure
    /// or interrupt. Used by ephemeral commands (`ocsq`, `oct`, `occmt`).
    pub ephemeral: bool,
    /// Show the shell commands (and other tool calls) the agent performs.
    /// Enabled by `ocsh` so the user can see what is executed; other commands
    /// keep stdout limited to the final result.
    pub show_tool_calls: bool,
}

pub struct RunResult {
    pub session_id: Option<String>,
    #[allow(dead_code)]
    pub text: String,
    #[allow(dead_code)]
    pub exit_code: i32,
}

/// The single entry point for `opencode run`.
///
/// Commands never build `opencode` invocations themselves and never deal with
/// the raw JSON event schema.
pub struct Runner {
    debug: bool,
}

impl Runner {
    pub fn new(debug: bool) -> Self {
        Self { debug }
    }

    pub async fn run(&self, options: &RunOptions) -> Result<RunResult> {
        let mut cmd = Command::new("opencode");
        cmd.arg("run").arg("--format").arg("json");
        if let Some(model) = &options.model {
            cmd.arg("--model").arg(model);
        }
        if let Some(session_id) = &options.session_id {
            cmd.arg("--session").arg(session_id);
        }
        if let Some(directory) = &options.directory {
            // OpenCode v2 resolves its workspace from `PWD` rather than from
            // `getcwd()`, so both must be set when running in another directory.
            cmd.current_dir(directory);
            cmd.env("PWD", directory);
        }
        cmd.arg(&options.prompt);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        if self.debug {
            let dir = options
                .directory
                .as_deref()
                .map(|dir| dir.display().to_string())
                .unwrap_or_else(|| ".".to_string());
            let model = options
                .model
                .as_deref()
                .map(|model| format!(" --model {}", model))
                .unwrap_or_default();
            let session = options
                .session_id
                .as_deref()
                .map(|id| format!(" --session {}", id))
                .unwrap_or_default();
            eprintln!("[debug] cwd: {}", dir);
            eprintln!(
                "[debug] opencode run --format json{}{} <prompt>",
                model, session
            );
            eprintln!("[debug] prompt:\n{}", options.prompt);
        }

        let mut child = cmd.spawn().map_err(|err| match err.kind() {
            io::ErrorKind::NotFound => OcError::OpencodeNotFound,
            _ => OcError::Io(err),
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            OcError::Message(
                tr(
                    "无法读取 opencode 输出",
                    "Failed to capture opencode output",
                )
                .to_string(),
            )
        })?;

        let mut reader = BufReader::new(stdout);
        let mut interrupt = Interrupt::new()?;
        let mut ctx = RunContext::default();
        let mut preview = Preview::new();
        let mut line = String::new();

        loop {
            line.clear();
            tokio::select! {
                read = reader.read_line(&mut line) => {
                    match read {
                        Ok(0) => break,
                        Ok(_) => self.handle_line(&mut ctx, &mut preview, options, line.trim_end()),
                        Err(err) => {
                            let _ = child.start_kill();
                            let _ = child.wait().await;
                            preview.finish_line();
                            self.cleanup_ephemeral(options, &ctx).await;
                            return Err(err.into());
                        }
                    }
                }
                _ = interrupt.wait() => {
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                    preview.finish_line();
                    self.cleanup_ephemeral(options, &ctx).await;
                    return Err(OcError::Interrupted);
                }
            }
        }

        let status = child.wait().await?;
        self.cleanup_ephemeral(options, &ctx).await;

        let RunContext {
            session_id,
            text,
            final_text,
            last_block,
            error_message,
            session_not_found,
            ..
        } = ctx;

        if session_not_found {
            preview.finish_line();
            return Err(OcError::SessionNotFound(session_id.unwrap_or_default()));
        }
        if let Some(message) = error_message {
            preview.finish_line();
            return Err(OcError::Message(message));
        }
        if !status.success() {
            preview.finish_line();
            return Err(OcError::ExitCode(status.code().unwrap_or(-1)));
        }

        let result_text = pick_result_text(final_text, last_block, text);

        if result_text.is_empty() {
            // Nothing new to show; keep the streamed preview.
            preview.finish_line();
        } else {
            // Replace the dimmed streaming preview with the final result.
            preview.clear();
            ui::print_final(&result_text);
        }

        Ok(RunResult {
            session_id,
            text: result_text,
            exit_code: status.code().unwrap_or(0),
        })
    }

    fn handle_line(
        &self,
        ctx: &mut RunContext,
        preview: &mut Preview,
        options: &RunOptions,
        line: &str,
    ) {
        if line.is_empty() {
            return;
        }

        let event: OpenCodeEvent = match serde_json::from_str(line) {
            Ok(event) => event,
            // Anything that is not one NDJSON event is forwarded to stderr so
            // it is never silently lost.
            Err(_) => {
                eprintln!("{}", line);
                return;
            }
        };

        if self.debug {
            eprintln!("[debug] {}", line);
        }

        if ctx.session_id.is_none() && !event.session_id.is_empty() {
            ctx.session_id = Some(event.session_id.clone());
            if self.debug {
                eprintln!("[debug] session: {}", event.session_id);
            }
        }

        if event.is_text() {
            let text = event.text().unwrap_or_default();
            if !text.is_empty() {
                preview.write(text);
                ctx.text.push_str(text);
                ctx.final_text.push_str(text);
            }
        } else if event.is_tool_use() {
            if options.show_tool_calls {
                show_tool_call(ctx, preview, &event);
            }
            // Text before a tool call is interim commentary, not the answer.
            if !ctx.final_text.is_empty() {
                ctx.last_block = std::mem::take(&mut ctx.final_text);
            }
        } else if event.is_error() {
            let message = event.error_message().map(str::to_owned).unwrap_or_else(|| {
                tr("OpenCode 执行出错", "OpenCode reported an error").to_string()
            });
            if message.to_lowercase().contains("session not found") {
                ctx.session_not_found = true;
            }
            if ctx.error_message.is_none() {
                ctx.error_message = Some(message);
            }
        }
    }

    async fn cleanup_ephemeral(&self, options: &RunOptions, ctx: &RunContext) {
        if !options.ephemeral {
            return;
        }

        match ctx.session_id.as_deref() {
            Some(session_id) => {
                if self.debug {
                    eprintln!("[debug] deleting ephemeral session {}", session_id);
                }
                match delete_opencode_session(session_id).await {
                    Ok(()) => {
                        if self.debug {
                            eprintln!("[debug] session deleted");
                        }
                    }
                    Err(err) => eprintln!("⚠️  {}", err),
                }
            }
            None => {
                eprintln!(
                    "⚠️  {}",
                    tr(
                        "未能获取本次 OpenCode session ID，无法自动清理会话。",
                        "Could not determine the OpenCode session ID; the session was not cleaned up."
                    )
                );
            }
        }
    }
}

/// Show one tool call as a permanent line, replacing the dimmed text preview.
///
/// The interim commentary that preceded the tool call is erased first, so what
/// remains on screen is the activity log (commands and tool calls) plus, at the
/// end, the final result. Commands are written to stderr, so they stay visible
/// even when stdout is redirected. Each tool call is shown only once, even if
/// OpenCode emits it more than once while it changes state.
fn show_tool_call(ctx: &mut RunContext, preview: &mut Preview, event: &OpenCodeEvent) {
    if let Some(id) = event.tool_id() {
        if !ctx.shown_tools.insert(id.to_string()) {
            return;
        }
    }

    preview.clear();

    if let Some(command) = event.tool_command().filter(|c| !c.trim().is_empty()) {
        ui::print_command(command);
    } else if let Some(name) = event.tool_name() {
        ui::print_tool(name, event.tool_title());
    }
}

/// Pick what the user should see as the final result.
///
/// The text produced after the last tool call wins. If the run ended on a tool
/// call without new text, the previous completed text block is used; if there
/// was no tool call at all, the whole response is the answer.
fn pick_result_text(final_text: String, last_block: String, all_text: String) -> String {
    if !final_text.is_empty() {
        final_text
    } else if !last_block.is_empty() {
        last_block
    } else {
        all_text
    }
}

#[derive(Default)]
struct RunContext {
    session_id: Option<String>,
    /// Every text part received during the run.
    text: String,
    /// Text received since the last tool call.
    final_text: String,
    /// The most recent text block completed before a tool call.
    last_block: String,
    error_message: Option<String>,
    session_not_found: bool,
    /// Tool call ids already shown, so each is printed once.
    shown_tools: std::collections::HashSet<String>,
}

/// SIGINT / SIGTERM handling while an `opencode` child process is running.
struct Interrupt {
    #[cfg(unix)]
    sigint: tokio::signal::unix::Signal,
    #[cfg(unix)]
    sigterm: tokio::signal::unix::Signal,
}

impl Interrupt {
    fn new() -> Result<Self> {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};

            Ok(Self {
                sigint: signal(SignalKind::interrupt())?,
                sigterm: signal(SignalKind::terminate())?,
            })
        }
        #[cfg(not(unix))]
        {
            Ok(Self {})
        }
    }

    async fn wait(&mut self) {
        #[cfg(unix)]
        {
            tokio::select! {
                _ = self.sigint.recv() => {}
                _ = self.sigterm.recv() => {}
            }
        }
        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only used to drive the ignored live test; production code reads the
    /// model from the config file instead of hardcoding it.
    const TEST_MODEL: &str = "opencode-go/mimo-v2.6-flash";

    #[test]
    fn final_result_prefers_text_after_the_last_tool_call() {
        assert_eq!(
            pick_result_text("answer".into(), "previous".into(), "everything".into()),
            "answer"
        );
        assert_eq!(
            pick_result_text(String::new(), "previous".into(), "everything".into()),
            "previous"
        );
        assert_eq!(
            pick_result_text(String::new(), String::new(), "everything".into()),
            "everything"
        );
    }

    /// Phase 1 internal test: start, parse text, capture sessionID, continue
    /// the session, then delete it.
    ///
    /// Requires a logged-in OpenCode installation, so it is ignored by default:
    ///
    /// ```text
    /// cargo test -- --ignored live_roundtrip
    /// ```
    #[tokio::test]
    #[ignore = "requires a logged-in OpenCode installation"]
    async fn live_roundtrip() {
        let runner = Runner::new(true);
        let first = runner
            .run(&RunOptions {
                model: Some(TEST_MODEL.to_string()),
                session_id: None,
                directory: None,
                prompt: "只回答 hello".to_string(),
                ephemeral: false,
                show_tool_calls: false,
            })
            .await
            .expect("first run");

        let session_id = first.session_id.clone().expect("session id");
        assert_eq!(first.text.trim(), "hello");

        let second = runner
            .run(&RunOptions {
                model: Some(TEST_MODEL.to_string()),
                session_id: Some(session_id.clone()),
                directory: None,
                prompt: "你刚才回答了什么？只回答那个单词".to_string(),
                ephemeral: false,
                show_tool_calls: false,
            })
            .await
            .expect("second run");

        assert_eq!(second.session_id.as_deref(), Some(session_id.as_str()));
        assert_eq!(second.text.trim(), "hello");

        delete_opencode_session(&session_id)
            .await
            .expect("delete session");
    }
}
