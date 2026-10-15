use crate::cli::QArgs;
use crate::error::{OcError, Result};
use crate::i18n::{is_zh, tr};
use crate::opencode::runner::{RunOptions, Runner};
use crate::opencode::session::SessionStore;
use crate::opencode::{resolve_model, Command};
use crate::ui::{self, CommandTimer};

/// `ocq` — continuous quick Q&A.
///
/// A single global OpenCode session is reused for every invocation, so
/// follow-up questions keep their context. Message history and compaction are
/// entirely owned by OpenCode.
pub async fn execute(args: QArgs) -> Result<()> {
    let timer = CommandTimer::start();
    let store = SessionStore::new();

    if args.clear {
        clear_session(&store, true).await?;
        println!("{}", tr("问答历史已清除。", "Q&A history cleared."));
        return Ok(());
    }

    let question = args.question.join(" ");
    let question = question.trim();
    if question.is_empty() {
        print_usage();
        return Ok(());
    }

    if args.new {
        // Failures are already reported by `clear_session`; starting a fresh
        // session is still the right behavior.
        let _ = clear_session(&store, false).await;
    }

    let model = resolve_model(Command::Q, &args.model)?;
    let runner = Runner::new(args.debug);
    let prompt = build_prompt(question);

    let mut session_id = store.get_ocq()?;
    let mut retried = false;

    loop {
        let options = RunOptions {
            model: model.clone(),
            session_id: session_id.clone(),
            directory: None,
            prompt: prompt.clone(),
            ephemeral: false,
        };

        match runner.run(&options).await {
            Ok(result) => {
                if let Some(id) = result.session_id {
                    if session_id.as_deref() != Some(id.as_str()) {
                        store.set_ocq(&id)?;
                    }
                }
                break;
            }
            Err(OcError::SessionNotFound(_)) if !retried => {
                ui::warn(tr(
                    "会话已失效，正在创建新会话...",
                    "Session is no longer available; creating a new one...",
                ));
                store.clear_ocq()?;
                session_id = None;
                retried = true;
            }
            Err(err) => return Err(err),
        }
    }

    timer.print_elapsed();
    Ok(())
}

async fn clear_session(store: &SessionStore, strict: bool) -> Result<()> {
    if let Some(session_id) = store.get_ocq()? {
        super::delete_session(&session_id, strict).await?;
        store.clear_ocq()?;
    }
    Ok(())
}

fn build_prompt(question: &str) -> String {
    let instruction = tr(
        "请用 1-3 句话简洁回答下面的问题，直接给出结论，不要冗余。除非用户明确要求，否则不要使用任何工具。",
        "Answer the question below concisely in 1-3 sentences, conclusion first. Do not use tools unless explicitly asked.",
    );
    format!("{}\n\n{}", instruction, question)
}

fn print_usage() {
    if is_zh() {
        println!("用法: ocq [选项] <问题>");
        println!("  -n, --new         删除旧会话并开始新会话");
        println!("  --clear           清除问答会话后退出");
        println!("  -m, --model       覆盖配置文件中的模型");
        println!("  --debug           输出原始 OpenCode 事件");
    } else {
        println!("Usage: ocq [options] <question>");
        println!("  -n, --new         Delete the old session and start a new one");
        println!("  --clear           Clear the Q&A session and exit");
        println!("  -m, --model       Override the model from config.jsonc");
        println!("  --debug           Print raw OpenCode events");
    }
}
