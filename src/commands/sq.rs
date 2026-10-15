use crate::cli::SqArgs;
use crate::error::Result;
use crate::i18n::{is_zh, tr};
use crate::opencode::runner::{RunOptions, Runner};
use crate::opencode::{resolve_model, Command};
use crate::ui::CommandTimer;

/// `ocsq` — single question, no history.
///
/// The session is created for this one question and deleted afterwards.
pub async fn execute(args: SqArgs) -> Result<()> {
    let timer = CommandTimer::start();

    let question = args.question.join(" ");
    let question = question.trim();
    if question.is_empty() {
        print_usage();
        return Ok(());
    }

    let model = resolve_model(Command::Sq, &args.model)?;
    let runner = Runner::new(args.debug);
    let options = RunOptions {
        model,
        session_id: None,
        directory: None,
        prompt: build_prompt(question),
        ephemeral: true,
    };

    runner.run(&options).await?;

    timer.print_elapsed();
    Ok(())
}

fn build_prompt(question: &str) -> String {
    let instruction = tr(
        "这是一个独立问题，没有历史上下文。请用 1-3 句话简洁回答，直接给出结论，不要冗余。除非用户明确要求，否则不要使用任何工具。",
        "This is a standalone question without conversation history. Answer it concisely in 1-3 sentences, conclusion first. Do not use tools unless explicitly asked.",
    );
    format!("{}\n\n{}", instruction, question)
}

fn print_usage() {
    if is_zh() {
        println!("用法: ocsq [选项] <问题>");
        println!("  -m, --model       覆盖配置文件中的模型");
        println!("  --debug           输出原始 OpenCode 事件");
    } else {
        println!("Usage: ocsq [options] <question>");
        println!("  -m, --model       Override the model from config.jsonc");
        println!("  --debug           Print raw OpenCode events");
    }
}
