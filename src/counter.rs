#[derive(Debug, Default)]
pub struct Counter {
    counter: u64,

    pub frame: u64,
    pub x: u64,
    pub y: u64,
}

impl Counter {
    pub fn elapse(&mut self, clock: u64) {
        self.counter += clock;
    }

    pub fn now(&self) -> u64 {
        self.counter
    }
}
