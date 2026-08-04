use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "aiclik", version, about = "AI CLI Kits in Rust")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Concise Q&A (maintains conversation context)
    #[command(name = "q", alias = "ask")]
    Q {
        /// Clear context memory and start a new conversation
        #[arg(short = 'n', long = "new")]
        new: bool,

        /// Question content
        #[arg(required = true)]
        query: Vec<String>,
    },

    /// Natural language to Shell command with confirmation
    #[command(name = "shell")]
    Shell {
        /// Command description
        #[arg(required = true)]
        query: Vec<String>,
    },

    /// Translation between Chinese & English and spell check
    #[command(name = "trans", alias = "ait")]
    Trans {
        /// Text to translate or correct
        #[arg(required = true)]
        query: Vec<String>,
    },
}
