use std::io::{self, IsTerminal, Write};

use colored::Colorize;
use unicode_width::UnicodeWidthChar;

/// Live preview of streamed OpenCode text.
///
/// While a run is in progress the incoming text parts are printed in a dim
/// color so the user can follow along. Once the final result is known,
/// [`Preview::clear`] erases everything that was previewed, so only the final
/// result remains on screen.
///
/// Nothing is previewed when stdout is not a terminal; in that case the final
/// result is printed once, at the end.
pub struct Preview {
    enabled: bool,
    rows: usize,
    col: usize,
    width: usize,
    printed: bool,
    last_newline: bool,
}

impl Preview {
    pub fn new() -> Self {
        Self {
            enabled: io::stdout().is_terminal(),
            rows: 0,
            col: 0,
            width: terminal_width().unwrap_or(80),
            printed: false,
            last_newline: true,
        }
    }

    /// Print one streamed chunk dimmed and remember how much room it took.
    pub fn write(&mut self, text: &str) {
        if !self.enabled || text.is_empty() {
            return;
        }

        let mut stdout = io::stdout().lock();
        let _ = write!(stdout, "{}", text.bright_black());
        let _ = stdout.flush();

        self.printed = true;
        self.last_newline = text.ends_with('\n');
        self.track(text);
    }

    /// Erase everything written since the preview started.
    pub fn clear(&mut self) {
        if !self.enabled || (self.rows == 0 && self.col == 0) {
            return;
        }

        // Clear the current line first, then walk back up over the previous
        // ones, so a preview without a trailing newline is fully removed.
        let mut escape = String::from("\r\x1b[2K");
        for _ in 0..self.rows {
            escape.push_str("\x1b[1A\x1b[2K");
        }

        let mut stdout = io::stdout().lock();
        let _ = write!(stdout, "{}", escape);
        let _ = stdout.flush();

        self.rows = 0;
        self.col = 0;
        self.printed = false;
        self.last_newline = true;
    }

    /// Ensure the cursor is on a fresh line after a preview that did not end
    /// with a newline. Used when the preview is kept (interrupt or error).
    pub fn finish_line(&mut self) {
        if !self.enabled || !self.printed || self.last_newline {
            return;
        }

        let mut stdout = io::stdout().lock();
        let _ = writeln!(stdout);
        let _ = stdout.flush();

        self.rows += 1;
        self.col = 0;
        self.last_newline = true;
    }

    /// Update the cursor position for a chunk without printing it.
    fn track(&mut self, text: &str) {
        for ch in text.chars() {
            match ch {
                '\n' => {
                    self.rows += 1;
                    self.col = 0;
                }
                '\r' => self.col = 0,
                '\t' => {
                    let next = (self.col / 8 + 1) * 8;
                    if next >= self.width {
                        self.rows += 1;
                        self.col = next % self.width;
                    } else {
                        self.col = next;
                    }
                }
                ch if (ch as u32) < 0x20 => {}
                ch => {
                    let width = UnicodeWidthChar::width(ch).unwrap_or(0);
                    if width == 0 {
                        continue;
                    }
                    if self.col + width > self.width {
                        self.rows += 1;
                        self.col = width;
                    } else {
                        self.col += width;
                    }
                }
            }
        }
    }
}

#[cfg(unix)]
fn terminal_width() -> Option<usize> {
    let mut size: libc::winsize = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut size) };
    if rc == 0 && size.ws_col > 0 {
        Some(size.ws_col as usize)
    } else {
        None
    }
}

#[cfg(not(unix))]
fn terminal_width() -> Option<usize> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preview(width: usize) -> Preview {
        Preview {
            enabled: true,
            rows: 0,
            col: 0,
            width,
            printed: false,
            last_newline: true,
        }
    }

    #[test]
    fn tracks_wrapping() {
        let mut preview = preview(10);
        preview.track("abcdefghijklmno");
        assert_eq!(preview.rows, 1);
        assert_eq!(preview.col, 5);
    }

    #[test]
    fn exact_fit_does_not_add_a_row() {
        let mut preview = preview(10);
        preview.track("abcdefghij");
        assert_eq!(preview.rows, 0);
        assert_eq!(preview.col, 10);
    }

    #[test]
    fn tracks_newlines() {
        let mut preview = preview(10);
        preview.track("ab\ncd\n");
        assert_eq!(preview.rows, 2);
        assert_eq!(preview.col, 0);
    }

    #[test]
    fn tracks_wide_characters() {
        let mut preview = preview(10);
        preview.track("中文中文中文");
        assert_eq!(preview.rows, 1);
        assert_eq!(preview.col, 2);
    }
}
