use context::{Bus, Cpu, Ppu, Timing};
use log::debug;

mod bus;
mod cartridge;
mod context;
mod counter;
mod cpu;
mod interrupt;
mod ppu;

pub struct Snes {
    pub context: context::Context,
}

impl Snes {
    pub fn new(rom: Vec<u8>) -> Snes {
        Snes {
            context: context::Context::new(rom),
        }
    }

    pub fn run(&mut self) {
        loop {
            self.context.exce_one();
        }
    }

    pub fn exec_frame(&mut self) {
        let frame = self.context.inner1.inner2.ppu.frame_number;
        while frame == self.context.inner1.inner2.ppu.frame_number {
            debug!("Before exec_one: now: {}", self.context.inner1.inner2.now());
            self.context.exce_one();
            debug!("After exce_one: now: {}", self.context.inner1.inner2.now());
            self.context.inner1.inner2.ppu_tick();
            debug!("After ppu_tick: now: {}", self.context.inner1.inner2.now());
            self.context.inner1.bus_tick();
            debug!("After bus_tick: now: {}", self.context.inner1.inner2.now());
        }
    }
}
