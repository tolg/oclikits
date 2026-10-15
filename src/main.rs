mod cli;
mod commands;
mod config;
mod error;
mod i18n;
mod opencode;
mod ui;

use std::env;
use std::ffi::OsStr;
use std::path::Path;
use std::process::exit;

use clap::{CommandFactory, FromArgMatches};

use cli::{Cli, Commands, CommitArgs, QArgs, ShellArgs, SqArgs, TransArgs};
use error::{OcError, Result};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let exec_name = args
        .first()
        .map(|arg| {
            Path::new(arg)
                .file_name()
                .and_then(OsStr::to_str)
                .unwrap_or("oc")
        })
        .unwrap_or("oc")
        .to_string();

    match run(&exec_name, &args).await {
        Ok(()) => {}
        Err(OcError::Interrupted) => {
            eprintln!("{}", i18n::tr("❌ 已中断。", "❌ Interrupted."));
            exit(130);
        }
        Err(err) => {
            eprintln!("❌ {}", err);
            exit(1);
        }
    }
}

async fn run(exec_name: &str, args: &[String]) -> Result<()> {
    match exec_name {
        "ocq" => commands::q::execute(parse_alias::<QArgs>("q", "ocq", args)).await,
        "ocsq" => commands::sq::execute(parse_alias::<SqArgs>("sq", "ocsq", args)).await,
        "oct" => commands::trans::execute(parse_alias::<TransArgs>("trans", "oct", args)).await,
        "octrans" => {
            commands::trans::execute(parse_alias::<TransArgs>("trans", "octrans", args)).await
        }
        "ocsh" => commands::shell::execute(parse_alias::<ShellArgs>("shell", "ocsh", args)).await,
        "occmt" => {
            commands::commit::execute(parse_alias::<CommitArgs>("commit", "occmt", args)).await
        }

        // Legacy names from aiclikits, kept with a one-time migration hint.
        "aiq" => {
            legacy_hint("aiq", "ocq");
            commands::q::execute(parse_alias::<QArgs>("q", "aiq", args)).await
        }
        "aiask" => {
            legacy_hint("aiask", "ocq");
            commands::q::execute(parse_alias::<QArgs>("q", "aiask", args)).await
        }
        "aish" => {
            legacy_hint("aish", "ocsh");
            commands::shell::execute(parse_alias::<ShellArgs>("shell", "aish", args)).await
        }
        "ait" => {
            legacy_hint("ait", "oct");
            commands::trans::execute(parse_alias::<TransArgs>("trans", "ait", args)).await
        }
        "aitrans" => {
            legacy_hint("aitrans", "oct");
            commands::trans::execute(parse_alias::<TransArgs>("trans", "aitrans", args)).await
        }

        // `oc <subcommand>` and any other name fall back to subcommand mode.
        _ => {
            let cli = parse_cli(args);
            match cli.command {
                Commands::Q(args) => commands::q::execute(args).await,
                Commands::Sq(args) => commands::sq::execute(args).await,
                Commands::Trans(args) => commands::trans::execute(args).await,
                Commands::Shell(args) => commands::shell::execute(args).await,
                Commands::Commit(args) => commands::commit::execute(args).await,
                Commands::InitConfig => {
                    let path = config::config_path();
                    if config::write_config_template()? {
                        println!(
                            "{} {}",
                            i18n::tr(
                                "已创建模型配置模板（model 可留空，留空则使用 OpenCode 默认模型）：",
                                "Created model config template (model may be left empty to use OpenCode's default):"
                            ),
                            path.display()
                        );
                    } else {
                        println!(
                            "{} {}",
                            i18n::tr("模型配置已存在：", "Model config already exists:"),
                            path.display()
                        );
                    }
                    Ok(())
                }
            }
        }
    }
}

/// Parse one subcommand definition as a top-level command, so that a symlink
/// like `ocq` behaves exactly like `oc q` while showing its own name in help.
fn parse_alias<T: clap::Args>(subcommand: &str, exec_name: &'static str, args: &[String]) -> T {
    let mut cmd = build_cli();
    let command = cmd
        .find_subcommand_mut(subcommand)
        .expect("subcommand is defined")
        .clone()
        .name(exec_name)
        .bin_name(exec_name)
        .version(env!("CARGO_PKG_VERSION"));

    let matches = command.get_matches_from(args);
    T::from_arg_matches(&matches).unwrap_or_else(|err| err.exit())
}

fn parse_cli(args: &[String]) -> Cli {
    let matches = build_cli().get_matches_from(args);
    Cli::from_arg_matches(&matches).unwrap_or_else(|err| err.exit())
}

fn build_cli() -> clap::Command {
    let mut cmd = Cli::command();

    if i18n::is_zh() {
        cmd = cmd
            .about("基于 OpenCode 的命令行工具集")
            .mut_subcommand("q", |sub| {
                sub.about("连续快速问答（保持会话）")
                    .mut_arg("clear", |arg| arg.help("清除问答会话后退出"))
                    .mut_arg("new", |arg| arg.help("删除旧会话并开始新会话"))
                    .mut_arg("model", |arg| {
                        arg.help("指定 OpenCode 模型（覆盖配置文件），格式 provider/model")
                    })
                    .mut_arg("debug", |arg| arg.help("输出原始 OpenCode 事件"))
                    .mut_arg("question", |arg| arg.help("问题内容"))
            })
            .mut_subcommand("sq", |sub| {
                sub.about("单次问答（无历史，临时会话）")
                    .mut_arg("model", |arg| {
                        arg.help("指定 OpenCode 模型（覆盖配置文件），格式 provider/model")
                    })
                    .mut_arg("debug", |arg| arg.help("输出原始 OpenCode 事件"))
                    .mut_arg("question", |arg| arg.help("问题内容"))
            })
            .mut_subcommand("trans", |sub| {
                sub.about("中英翻译与错别字纠正")
                    .mut_arg("model", |arg| {
                        arg.help("指定 OpenCode 模型（覆盖配置文件），格式 provider/model")
                    })
                    .mut_arg("debug", |arg| arg.help("输出原始 OpenCode 事件"))
                    .mut_arg("text", |arg| arg.help("翻译或纠错文本"))
            })
            .mut_subcommand("shell", |sub| {
                sub.about("绑定当前目录的 Shell Agent")
                    .mut_arg("clear", |arg| arg.help("清除当前目录会话后退出"))
                    .mut_arg("new", |arg| arg.help("删除当前目录会话并开始新会话"))
                    .mut_arg("model", |arg| {
                        arg.help("指定 OpenCode 模型（覆盖配置文件），格式 provider/model")
                    })
                    .mut_arg("debug", |arg| arg.help("输出原始 OpenCode 事件"))
                    .mut_arg("task", |arg| arg.help("任务描述"))
            })
            .mut_subcommand("commit", |sub| {
                sub.about("通过 OpenCode 提交当前 Git 修改")
                    .mut_arg("en", |arg| arg.help("提交信息使用英文"))
                    .mut_arg("zh", |arg| arg.help("提交信息使用简体中文"))
                    .mut_arg("model", |arg| {
                        arg.help("指定 OpenCode 模型（覆盖配置文件），格式 provider/model")
                    })
                    .mut_arg("debug", |arg| arg.help("输出原始 OpenCode 事件"))
            });
    }

    cmd
}

fn legacy_hint(old: &str, new: &str) {
    let zh = format!("提示：{} 已更名为 {}。", old, new);
    let en = format!("Note: {} has been renamed to {}.", old, new);
    ui::note(i18n::tr(&zh, &en));
}
