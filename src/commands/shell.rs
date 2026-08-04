use crate::client::ApiClient;
use crate::config::Config;
use crate::error::{AidError, Result};
use crate::timer::CommandTimer;
use crate::ui::{print_command_box, start_spinner};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

pub async fn execute(cfg: &Config, query_parts: Vec<String>) -> Result<()> {
    let timer = CommandTimer::start();
    let query = query_parts.join(" ");
    let query = query.trim();
    if query.is_empty() {
        println!("用法: aish <自然语言指令>");
        return Ok(());
    }

    let os_output = Command::new("uname")
        .arg("-s")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| env::consts::OS.to_string());

    let shell_path = env::var("SHELL").unwrap_or_else(|_| "zsh".to_string());
    let shell_name = Path::new(&shell_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("zsh");

    let system_prompt = format!(
        "You are a strict command-line translator for {} ({}).\n\
        RULES:\n\
        1. Output ONLY the raw executable command string.\n\
        2. DO NOT include markdown code blocks (```).\n\
        3. DO NOT output multiple options. Decide on the Single Best One.\n\
        4. DO NOT include any explanations or conversational text.\n\
        5. If multiple steps are required, combine them using '&&' or '|' or ';'.\n\
        6. Your output will be directly executed by 'eval'. Any non-code text will cause a crash.",
        os_output, shell_name
    );

    let pb = start_spinner("🤖 Thinking...");
    let client = ApiClient::new(cfg)?;
    let model = cfg.model_for("shell", "qwen3-coder-flash-2025-07-28");
    let temperature = cfg.temperature();

    let content_res = client
        .chat(&model, &system_prompt, query, temperature)
        .await;

    pb.finish_and_clear();

    let content = content_res?;
    let cmd = clean_command_output(&content);

    if cmd.is_empty() {
        return Err(AidError::Config("解析为空".to_string()));
    }

    print_command_box(&cmd);
    timer.print_elapsed();
    println!();

    print!("执行? [Y/n] ");
    io::stdout().flush().ok();

    let confirm = read_confirmation()?;
    if confirm {
        println!();
        let status = Command::new(&shell_path)
            .arg("-c")
            .arg(&cmd)
            .status()?;
        if !status.success() {
            if let Some(code) = status.code() {
                eprintln!("命令退出码: {}", code);
            }
        }
    } else {
        println!("\n已取消。");
    }

    Ok(())
}

fn clean_command_output(raw: &str) -> String {
    let mut lines = raw.lines();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let cleaned = trimmed
            .trim_start_matches("```")
            .trim_start_matches("```bash")
            .trim_start_matches("```sh")
            .trim_start_matches('`')
            .trim_end_matches("```")
            .trim_end_matches('`')
            .trim();

        if !cleaned.is_empty() {
            return cleaned.to_string();
        }
    }
    String::new()
}

fn read_confirmation() -> Result<bool> {
    enable_raw_mode()?;
    let result = loop {
        if let Event::Key(key_event) = event::read()? {
            if key_event.kind == KeyEventKind::Press {
                match key_event.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => break true,
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => break false,
                    _ => break true, // Default to true on other keypress/enter like in original script
                }
            }
        }
    };
    disable_raw_mode()?;
    Ok(result)
}
