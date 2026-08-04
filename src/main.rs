mod cli;
mod client;
mod config;

mod commands;
mod error;
mod history;
mod timer;
mod ui;

use clap::Parser;
use cli::{Cli, Commands};
use config::Config;
use std::env;
use std::ffi::OsStr;
use std::path::Path;
use std::process::exit;

#[derive(Parser, Debug)]
#[command(name = "aiq", about = "简洁问答 (保持历史上下文)")]
struct AliasQ {
    /// 清除上下文记忆并开始新对话
    #[arg(short = 'n', long = "new")]
    new: bool,

    /// 问题内容
    #[arg(required = true)]
    query: Vec<String>,
}

#[derive(Parser, Debug)]
#[command(name = "aish", about = "自然语言指令转 Shell 并交互确认")]
struct AliasShell {
    /// 指令描述
    #[arg(required = true)]
    query: Vec<String>,
}

#[derive(Parser, Debug)]
#[command(name = "aitrans", about = "中英翻译与错别字纠正")]
struct AliasTrans {
    /// 翻译或纠错文本
    #[arg(required = true)]
    query: Vec<String>,
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let exec_name = args
        .first()
        .map(|s| Path::new(s).file_name().and_then(OsStr::to_str).unwrap_or("aid"))
        .unwrap_or("aid");

    let cfg = Config::load();

    let result = match exec_name {
        "aiq" | "aiask" => {
            let parsed = AliasQ::parse_from(&args);
            commands::q::execute(&cfg, parsed.new, parsed.query).await
        }
        "aish" => {
            let parsed = AliasShell::parse_from(&args);
            commands::shell::execute(&cfg, parsed.query).await
        }
        "ait" | "aitrans" => {
            let parsed = AliasTrans::parse_from(&args);
            commands::trans::execute(&cfg, parsed.query).await
        }
        _ => {
            let cli = Cli::parse_from(&args);
            match cli.command {
                Commands::Q { new, query } => commands::q::execute(&cfg, new, query).await,
                Commands::Shell { query } => commands::shell::execute(&cfg, query).await,
                Commands::Trans { query } => commands::trans::execute(&cfg, query).await,
            }
        }
    };

    if let Err(err) = result {
        eprintln!("❌ {}", err);
        exit(1);
    }
}
