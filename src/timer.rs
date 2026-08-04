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
        println!("(用时 {:.2}s)", self.elapsed_sec());
    }
}
