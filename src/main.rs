mod cli;
mod client;
mod commands;
mod config;
mod error;
mod history;
mod i18n;
mod timer;
mod ui;

use clap::{CommandFactory, FromArgMatches, Parser};
use cli::{Cli, Commands};
use config::Config;
use std::env;
use std::ffi::OsStr;
use std::path::Path;
use std::process::exit;

#[derive(Parser, Debug)]
#[command(name = "aiq", about = "Concise Q&A (maintains conversation context)")]
struct AliasQ {
    /// Clear context memory and start a new conversation
    #[arg(short = 'n', long = "new")]
    new: bool,

    /// Question content
    #[arg(required = true)]
    query: Vec<String>,
}

#[derive(Parser, Debug)]
#[command(
    name = "aish",
    about = "Natural language to Shell command with confirmation"
)]
struct AliasShell {
    /// Command description
    #[arg(required = true)]
    query: Vec<String>,
}

#[derive(Parser, Debug)]
#[command(
    name = "aitrans",
    about = "Translation between Chinese & English and spell check"
)]
struct AliasTrans {
    /// Text to translate or correct
    #[arg(required = true)]
    query: Vec<String>,
}

fn parse_alias_q(args: &[String]) -> AliasQ {
    let mut cmd = AliasQ::command();
    if i18n::is_zh() {
        cmd = cmd
            .about("简洁问答 (保持历史上下文)")
            .mut_arg("new", |a| a.help("清除上下文记忆并开始新对话"))
            .mut_arg("query", |a| a.help("问题内容"));
    } else {
        cmd = cmd
            .about("Concise Q&A (maintains conversation context)")
            .mut_arg("new", |a| {
                a.help("Clear context memory and start a new conversation")
            })
            .mut_arg("query", |a| a.help("Question content"));
    }
    let matches = cmd.get_matches_from(args);
    AliasQ::from_arg_matches(&matches).unwrap_or_else(|err| err.exit())
}

fn parse_alias_shell(args: &[String]) -> AliasShell {
    let mut cmd = AliasShell::command();
    if i18n::is_zh() {
        cmd = cmd
            .about("自然语言指令转 Shell 并交互确认")
            .mut_arg("query", |a| a.help("指令描述"));
    } else {
        cmd = cmd
            .about("Natural language to Shell command with confirmation")
            .mut_arg("query", |a| a.help("Command description"));
    }
    let matches = cmd.get_matches_from(args);
    AliasShell::from_arg_matches(&matches).unwrap_or_else(|err| err.exit())
}

fn parse_alias_trans(args: &[String]) -> AliasTrans {
    let mut cmd = AliasTrans::command();
    if i18n::is_zh() {
        cmd = cmd
            .about("中英翻译与错别字纠正")
            .mut_arg("query", |a| a.help("翻译或纠错文本"));
    } else {
        cmd = cmd
            .about("Translation between Chinese & English and spell check")
            .mut_arg("query", |a| a.help("Text to translate or correct"));
    }
    let matches = cmd.get_matches_from(args);
    AliasTrans::from_arg_matches(&matches).unwrap_or_else(|err| err.exit())
}

fn parse_cli(args: &[String]) -> Cli {
    let mut cmd = Cli::command();
    if i18n::is_zh() {
        cmd = cmd
            .about("AI CLI Kits in Rust")
            .mut_subcommand("q", |sub| {
                sub.about("简洁问答 (保持历史上下文)")
                    .mut_arg("new", |a| a.help("清除上下文记忆并开始新对话"))
                    .mut_arg("query", |a| a.help("问题内容"))
            })
            .mut_subcommand("shell", |sub| {
                sub.about("自然语言指令转 Shell 并交互确认")
                    .mut_arg("query", |a| a.help("指令描述"))
            })
            .mut_subcommand("trans", |sub| {
                sub.about("中英翻译与错别字纠正")
                    .mut_arg("query", |a| a.help("翻译或纠错文本"))
            });
    } else {
        cmd = cmd
            .about("AI CLI Kits in Rust")
            .mut_subcommand("q", |sub| {
                sub.about("Concise Q&A (maintains conversation context)")
                    .mut_arg("new", |a| {
                        a.help("Clear context memory and start a new conversation")
                    })
                    .mut_arg("query", |a| a.help("Question content"))
            })
            .mut_subcommand("shell", |sub| {
                sub.about("Natural language to Shell command with confirmation")
                    .mut_arg("query", |a| a.help("Command description"))
            })
            .mut_subcommand("trans", |sub| {
                sub.about("Translation between Chinese & English and spell check")
                    .mut_arg("query", |a| a.help("Text to translate or correct"))
            });
    }
    let matches = cmd.get_matches_from(args);
    Cli::from_arg_matches(&matches).unwrap_or_else(|err| err.exit())
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
            let parsed = parse_alias_q(&args);
            commands::q::execute(&cfg, parsed.new, parsed.query).await
        }
        "aish" => {
            let parsed = parse_alias_shell(&args);
            commands::shell::execute(&cfg, parsed.query).await
        }
        "ait" | "aitrans" => {
            let parsed = parse_alias_trans(&args);
            commands::trans::execute(&cfg, parsed.query).await
        }
        _ => {
            let cli = parse_cli(&args);
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
