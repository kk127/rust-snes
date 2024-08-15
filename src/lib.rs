use context::Cpu;

mod bus;
mod cartridge;
mod context;
mod counter;
mod cpu;
mod ppu;

pub struct Snes {
    context: context::Context,
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
}
