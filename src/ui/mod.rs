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

/// Print a shell command executed by an agent, prefixed with `$`.
///
/// Written to stderr: it is activity information, so it stays visible even
/// when stdout is redirected, and never pollutes the command's real output.
/// Multi-line commands are printed with continuation lines indented so the
/// whole command stays readable.
pub fn print_command(command: &str) {
    let command = command.trim_end();
    if command.is_empty() {
        return;
    }

    let mut stderr = io::stderr().lock();
    for (index, line) in command.lines().enumerate() {
        if index == 0 {
            let _ = writeln!(stderr, "{} {}", "$".green().bold(), line.bright_white());
        } else {
            let _ = writeln!(stderr, "  {}", line.bright_white());
        }
    }
    let _ = stderr.flush();
}

/// Print a non-shell tool call, dimmed, so file and search activity is visible.
///
/// `detail` is only printed when it adds information beyond the tool name.
pub fn print_tool(name: &str, detail: Option<&str>) {
    let mut stderr = io::stderr().lock();
    match detail.map(str::trim) {
        Some(detail) if !detail.is_empty() && detail != name => {
            let _ = writeln!(
                stderr,
                "{} {} {}",
                "·".bright_black(),
                name.bright_black(),
                detail.bright_black()
            );
        }
        _ => {
            let _ = writeln!(stderr, "{} {}", "·".bright_black(), name.bright_black());
        }
    }
    let _ = stderr.flush();
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
