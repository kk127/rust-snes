use context::{Bus, Cpu, Ppu, Spc};
pub use controller::Key;

mod bus;
mod cartridge;
mod context;
mod controller;
mod counter;
mod cpu;
mod dsp;
mod interrupt;
mod ppu;
mod spc;

pub struct Snes {
    pub context: context::Context,
}

impl Snes {
    pub fn new(rom: Vec<u8>, backup: Option<Vec<u8>>) -> Snes {
        Snes {
            context: context::Context::new(rom, backup),
        }
    }

    pub fn run(&mut self) {
        loop {
            self.context.exce_one();
        }
    }

    pub fn set_keys(&mut self, keys: [Vec<Key>; 4]) {
        self.context.inner1.set_keys(keys);
    }

    pub fn exec_frame(&mut self) {
        let frame = self.context.inner1.inner2.ppu.frame_number;
        self.context.inner1.inner2.clear_audio_buffer();
        while frame == self.context.inner1.inner2.ppu.frame_number {
            self.context.exce_one();
            self.context.inner1.inner2.ppu_tick();
            self.context.inner1.inner2.spc_tick();
            self.context.inner1.bus_tick();
        }
    }

    pub fn backup(&self) -> Option<Vec<u8>> {
        self.context.inner1.inner2.cartridge.backup()
    }
}
