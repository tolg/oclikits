use std::path::{Path, PathBuf};

use crate::cli::ShellArgs;
use crate::error::{OcError, Result};
use crate::i18n::{is_zh, tr};
use crate::opencode::runner::{RunOptions, Runner};
use crate::opencode::session::SessionStore;
use crate::opencode::{resolve_model, Command};
use crate::ui::{self, CommandTimer};

/// `ocsh` — a lightweight shell agent bound to the current directory.
///
/// Exactly one OpenCode session exists per canonical working directory. When
/// the session is reused, the agent keeps its context; a different directory
/// always starts with a different session.
pub async fn execute(args: ShellArgs) -> Result<()> {
    let timer = CommandTimer::start();
    let store = SessionStore::new();
    let directory = current_directory()?;

    if args.clear {
        clear_session(&store, &directory, true).await?;
        println!(
            "{}",
            tr(
                "当前目录的 Shell 会话已清除。",
                "Shell session for this directory cleared."
            )
        );
        return Ok(());
    }

    let task = args.task.join(" ");
    let task = task.trim();
    if task.is_empty() {
        print_usage();
        return Ok(());
    }

    if args.new {
        // Failures are already reported by `clear_session`; starting a fresh
        // session is still the right behavior.
        let _ = clear_session(&store, &directory, false).await;
    }

    let model = resolve_model(Command::Shell, &args.model)?;
    let runner = Runner::new(args.debug);
    let prompt = build_prompt(&directory, task);

    let mut session_id = store.get_ocsh(&directory)?;
    let mut retried = false;

    loop {
        let options = RunOptions {
            model: model.clone(),
            session_id: session_id.clone(),
            directory: Some(directory.clone()),
            prompt: prompt.clone(),
            ephemeral: false,
            show_tool_calls: true,
        };

        match runner.run(&options).await {
            Ok(result) => {
                if let Some(id) = result.session_id {
                    if session_id.as_deref() != Some(id.as_str()) {
                        store.set_ocsh(&directory, &id)?;
                    }
                }
                break;
            }
            Err(OcError::SessionNotFound(_)) if !retried => {
                ui::warn(tr(
                    "会话已失效，正在创建新会话...",
                    "Session is no longer available; creating a new one...",
                ));
                store.clear_ocsh(&directory)?;
                session_id = None;
                retried = true;
            }
            Err(err) => return Err(err),
        }
    }

    timer.print_elapsed();
    Ok(())
}

async fn clear_session(store: &SessionStore, directory: &Path, strict: bool) -> Result<()> {
    if let Some(session_id) = store.get_ocsh(directory)? {
        super::delete_session(&session_id, strict).await?;
        store.clear_ocsh(directory)?;
    }
    Ok(())
}

fn current_directory() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    Ok(std::fs::canonicalize(&cwd).unwrap_or(cwd))
}

fn build_prompt(directory: &Path, task: &str) -> String {
    let os = std::env::consts::OS;
    let shell = std::env::var("SHELL")
        .ok()
        .and_then(|shell| {
            Path::new(&shell)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "sh".to_string());
    let directory = directory.display();

    if is_zh() {
        format!(
            "你是一个运行在终端中的 Shell Agent。\n\n当前工作目录：{}\n操作系统：{}\n默认 Shell：{}\n\n请使用 Shell 和文件工具调查并完成用户的任务。优先使用只读命令进行调查，最后简洁汇报结果。\n\n任务：\n{}",
            directory, os, shell, task
        )
    } else {
        format!(
            "You are a shell agent running in a terminal.\n\nWorking directory: {}\nOperating system: {}\nDefault shell: {}\n\nInvestigate and complete the user's task using shell and file tools. Prefer read-only commands for investigation and finish with a concise report.\n\nTask:\n{}",
            directory, os, shell, task
        )
    }
}

fn print_usage() {
    if is_zh() {
        println!("用法: ocsh [选项] <任务>");
        println!("  -n, --new         删除当前目录会话并开始新会话");
        println!("  --clear           清除当前目录会话后退出");
        println!("  -m, --model       覆盖配置文件中的模型");
        println!("  --debug           输出原始 OpenCode 事件");
    } else {
        println!("Usage: ocsh [options] <task>");
        println!("  -n, --new         Delete the session of this directory and start a new one");
        println!("  --clear           Clear the session of this directory and exit");
        println!("  -m, --model       Override the model from config.jsonc");
        println!("  --debug           Print raw OpenCode events");
    }
}
