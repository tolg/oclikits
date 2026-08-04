use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "aiclik", version, about = "AI CLI Kits in Rust")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// 简洁问答 (保持历史上下文)
    #[command(name = "q", alias = "ask")]
    Q {
        /// 清除上下文记忆并开始新对话
        #[arg(short = 'n', long = "new")]
        new: bool,

        /// 问题内容
        #[arg(required = true)]
        query: Vec<String>,
    },

    /// 自然语言指令转 Shell 并交互确认
    #[command(name = "shell")]
    Shell {
        /// 指令描述
        #[arg(required = true)]
        query: Vec<String>,
    },

    /// 中英翻译与错别字纠正
    #[command(name = "trans", alias = "ait")]
    Trans {
        /// 翻译或纠错文本
        #[arg(required = true)]
        query: Vec<String>,
    },
}
