use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "oc",
    version,
    about = "OpenCode CLI Kits",
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Continuous quick Q&A (persistent OpenCode session)
    #[command(name = "q", alias = "ask")]
    Q(QArgs),

    /// Single question without history (ephemeral OpenCode session)
    #[command(name = "sq", alias = "single")]
    Sq(SqArgs),

    /// Translate between Chinese and English
    #[command(name = "trans", alias = "t")]
    Trans(TransArgs),

    /// Shell agent bound to the current directory
    #[command(name = "shell", alias = "sh")]
    Shell(ShellArgs),

    /// Commit the current Git changes through OpenCode
    #[command(name = "commit", alias = "cmt")]
    Commit(CommitArgs),

    /// Write the model config template to config.jsonc (used by install.sh)
    #[command(name = "init-config", hide = true)]
    InitConfig,
}

#[derive(Args, Debug)]
pub struct QArgs {
    /// Clear the Q&A session and exit
    #[arg(long = "clear")]
    pub clear: bool,

    /// Delete the old Q&A session and start a new one
    #[arg(short = 'n', long = "new")]
    pub new: bool,

    /// OpenCode model (overrides config.jsonc), in provider/model format
    #[arg(short = 'm', long = "model", value_name = "MODEL")]
    pub model: Option<String>,

    /// Print raw OpenCode events
    #[arg(long = "debug")]
    pub debug: bool,

    /// Question content
    #[arg(value_name = "QUESTION")]
    pub question: Vec<String>,
}

#[derive(Args, Debug)]
pub struct SqArgs {
    /// OpenCode model (overrides config.jsonc), in provider/model format
    #[arg(short = 'm', long = "model", value_name = "MODEL")]
    pub model: Option<String>,

    /// Print raw OpenCode events
    #[arg(long = "debug")]
    pub debug: bool,

    /// Question content
    #[arg(value_name = "QUESTION")]
    pub question: Vec<String>,
}

#[derive(Args, Debug)]
pub struct TransArgs {
    /// OpenCode model (overrides config.jsonc), in provider/model format
    #[arg(short = 'm', long = "model", value_name = "MODEL")]
    pub model: Option<String>,

    /// Print raw OpenCode events
    #[arg(long = "debug")]
    pub debug: bool,

    /// Text to translate or correct
    #[arg(value_name = "TEXT")]
    pub text: Vec<String>,
}

#[derive(Args, Debug)]
pub struct ShellArgs {
    /// Clear the session of the current directory and exit
    #[arg(long = "clear")]
    pub clear: bool,

    /// Delete the session of the current directory and start a new one
    #[arg(short = 'n', long = "new")]
    pub new: bool,

    /// OpenCode model (overrides config.jsonc), in provider/model format
    #[arg(short = 'm', long = "model", value_name = "MODEL")]
    pub model: Option<String>,

    /// Print raw OpenCode events
    #[arg(long = "debug")]
    pub debug: bool,

    /// Task description
    #[arg(value_name = "TASK")]
    pub task: Vec<String>,
}

#[derive(Args, Debug)]
pub struct CommitArgs {
    /// Write the commit message in English
    #[arg(long = "en", conflicts_with = "zh")]
    pub en: bool,

    /// Write the commit message in Simplified Chinese
    #[arg(long = "zh", conflicts_with = "en")]
    pub zh: bool,

    /// OpenCode model (overrides config.jsonc), in provider/model format
    #[arg(short = 'm', long = "model", value_name = "MODEL")]
    pub model: Option<String>,

    /// Print raw OpenCode events
    #[arg(long = "debug")]
    pub debug: bool,
}
