use context::{Bus, Cpu, Ppu};

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
        // println!("~~~~~~frame_number: {}", frame);
        // println!("ooooooooooooooooooooooooooooooooooooooo");
        while frame == self.context.inner1.inner2.ppu.frame_number {
            self.context.exce_one();
            self.context.inner1.inner2.ppu_tick();
            self.context.inner1.bus_tick();
            // println!(
            //     "ppu frame_number: {}",
            //     self.context.inner1.inner2.ppu.frame_number
            // );
        }
    }
}
