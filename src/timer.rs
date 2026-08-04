use std::time::Instant;

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
        if crate::i18n::is_zh() {
            println!("(用时 {:.2}s)", self.elapsed_sec());
        } else {
            println!("(Elapsed {:.2}s)", self.elapsed_sec());
        }
    }
}
