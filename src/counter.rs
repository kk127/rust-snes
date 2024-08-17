pub struct Counter {
    counter: u64,
}

impl Default for Counter {
    fn default() -> Counter {
        Counter { counter: 0 }
    }
}

impl Counter {
    pub fn elapse(&mut self, clock: u64) {
        self.counter += clock;
    }

    pub fn now(&self) -> u64 {
        self.counter
    }
}
