pub mod preview;

use std::io::{self, Write};
use std::time::Instant;

use colored::Colorize;

use crate::error::Result;
use crate::i18n::tr;

/// High precision elapsed time tracking for one command invocation.
pub struct CommandTimer {
    start: Instant,
}

impl CommandTimer {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed_sec(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    pub fn print_elapsed(&self) {
        let zh = format!("(用时 {:.2}s)", self.elapsed_sec());
        let en = format!("(Elapsed {:.2}s)", self.elapsed_sec());
        print_line(&tr(&zh, &en).dimmed().to_string());
    }
}

/// Print the final result to stdout, ensuring a trailing newline.
pub fn print_final(text: &str) {
    if text.is_empty() {
        return;
    }

    let mut stdout = io::stdout().lock();
    let _ = write!(stdout, "{}", text);
    if !text.ends_with('\n') {
        let _ = writeln!(stdout);
    }
    let _ = stdout.flush();
}

/// Print one plain line to stdout.
pub fn print_line(text: &str) {
    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "{}", text);
    let _ = stdout.flush();
}

/// Read a line from stdin and accept only `y` / `yes` (case-insensitive).
pub fn confirm(prompt: &str) -> Result<bool> {
    let mut stdout = io::stdout().lock();
    let _ = write!(stdout, "{}", prompt);
    let _ = stdout.flush();
    drop(stdout);

    let mut line = String::new();
    if io::stdin().read_line(&mut line)? == 0 {
        return Ok(false);
    }

    let answer = line.trim().to_lowercase();
    Ok(matches!(answer.as_str(), "y" | "yes"))
}

pub fn warn(message: &str) {
    eprintln!("{}", format!("⚠️  {}", message).yellow());
}

pub fn note(message: &str) {
    eprintln!("{}", message.cyan());
}
