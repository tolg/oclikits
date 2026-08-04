use crate::client::types::ChatMessage;
use crate::client::ApiClient;
use crate::config::Config;
use crate::error::Result;
use crate::history::HistoryManager;
use crate::timer::CommandTimer;
use std::io::{self, Write};

pub async fn execute(cfg: &Config, new: bool, query_parts: Vec<String>) -> Result<()> {
    let timer = CommandTimer::start();
    let history_mgr = HistoryManager::new();

    if new {
        history_mgr.clear();
        println!("🧹 记忆已清除");
    }

    let query = query_parts.join(" ");
    let query = query.trim();
    if query.is_empty() {
        println!("用法: aiq [选项] <问题>");
        println!("  -n, --new   清除上下文记忆并开始新对话");
        return Ok(());
    }

    let system_prompt = "用 1-3 句话简洁回答以下问题，不要冗余，直接给结论：";
    let history = history_mgr.load();

    let mut messages = vec![ChatMessage::system(system_prompt)];
    messages.extend(history);

    let user_msg = ChatMessage::user(query);
    messages.push(user_msg.clone());

    let client = ApiClient::new(cfg)?;
    let model = cfg.model_for("ask", "qwen3-flash");
    let temperature = cfg.temperature();

    let answer = client
        .chat_messages_stream(&model, messages, temperature, |chunk| {
            print!("{}", chunk);
            io::stdout().flush().ok();
        })
        .await?;

    println!();
    timer.print_elapsed();

    let assistant_msg = ChatMessage::assistant(answer.trim());
    let mut updated_history = history_mgr.load();
    updated_history.push(user_msg);
    updated_history.push(assistant_msg);
    history_mgr.save(updated_history);

    Ok(())
}
