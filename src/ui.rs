use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub fn start_spinner(msg: &'static str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner} {msg}")
            .unwrap(),
    );
    pb.set_message(msg);
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

pub fn print_command_box(cmd: &str) {
    println!("----------------------------------------");
    println!("{}", cmd.green().bold());
    println!("----------------------------------------");
}
