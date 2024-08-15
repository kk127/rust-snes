use crate::{bus, cartridge, counter, cpu, ppu};

// struct Context {
//     cpu: cpu::Cpu,
//     bus: bus::Bus,
//     ppu: ppu::Ppu,
//     timing: counter::Counter,
//     cartridge: cartridge::Cartridge,
// }

pub struct Context {
    cpu: cpu::Cpu,
    inner1: Inner1,
}

struct Inner1 {
    bus: bus::Bus,
    inner2: Inner2,
}

struct Inner2 {
    ppu: ppu::Ppu,
    cartridge: cartridge::Cartridge,
    inner: Inner3,
}
struct Inner3 {
    timing: counter::Counter,
}

// impl Context {
//     fn new(rom: Vec<u8>) -> Context {
//         Context {
//             cpu: cpu::Cpu::default(),
//             bus: bus::Bus::default(),
//             ppu: ppu::Ppu::default(),
//             timing: counter::Counter::default(),
//             cartridge: cartridge::Cartridge::new(rom),
//         }
//     }
// }

impl Context {
    pub fn new(rom: Vec<u8>) -> Context {
        let mut ctx = Context {
            cpu: cpu::Cpu::default(),
            inner1: Inner1 {
                bus: bus::Bus::default(),
                inner2: Inner2 {
                    ppu: ppu::Ppu::default(),
                    cartridge: cartridge::Cartridge::new(rom),
                    inner: Inner3 {
                        timing: counter::Counter::default(),
                    },
                },
            },
        };
        ctx.cpu.reset(&mut ctx.inner1);
        ctx
    }
}

impl Cpu for Context {
    fn exce_one(&mut self) {
        self.cpu.excecute_instruction(&mut self.inner1)
    }
    fn reset(&mut self) {
        self.cpu.reset(&mut self.inner1)
    }
}

impl Bus for Inner1 {
    fn bus_read(&mut self, addr: u32) -> u8 {
        self.bus.read(addr, &mut self.inner2)
    }

    fn bus_write(&mut self, addr: u32, data: u8) {
        self.bus.write(addr, data, &mut self.inner2)
    }
}

impl Timing for Inner1 {
    fn elapse(&mut self, clock: u64) {
        self.inner2.elapse(clock)
    }
}

impl Ppu for Inner2 {
    fn ppu_read(&mut self, addr: u32) -> u8 {
        self.ppu.read(addr, &mut self.inner)
    }

    fn ppu_write(&mut self, addr: u32, data: u8) {
        self.ppu.write(addr, data, &mut self.inner)
    }
}

impl Cartridge for Inner2 {
    fn cartridge_read(&mut self, addr: u32) -> u8 {
        self.cartridge.read(addr)
    }

    fn cartridge_write(&mut self, addr: u32, data: u8) {
        self.cartridge.write(addr, data)
    }
}

impl Timing for Inner2 {
    fn elapse(&mut self, clock: u64) {
        self.inner.elapse(clock)
    }
}

impl Timing for Inner3 {
    fn elapse(&mut self, clock: u64) {
        self.timing.elapse(clock)
    }
}

// impl Bus for Context {
//     fn bus_read(&mut self, addr: u32) -> u8 {
//         self.bus.read(addr, self)
//     }

//     fn bus_write(&mut self, addr: u32, data: u8) {
//         self.bus.write(addr, data, self)
//     }
// }

// impl Ppu for Context {
//     fn ppu_read(&mut self, addr: u32) -> u8 {
//         self.ppu.read(addr, self)
//     }

//     fn ppu_write(&mut self, addr: u32, data: u8) {
//         self.ppu.write(addr, data, self)
//     }
// }

// impl Timing for Context {
//     fn elapse(&mut self, clock: u64) {
//         self.timing.elapse(clock)
//     }
// }

// impl Cartridge for Context {
//     fn cartridge_read(&mut self, addr: u32) -> u8 {
//         self.cartridge.read(addr)
//     }

//     fn cartridge_write(&mut self, addr: u32, data: u8) {
//         self.cartridge.write(addr, data)
//     }
// }

pub trait Cpu {
    fn exce_one(&mut self);
    fn reset(&mut self);
}

pub trait Bus {
    fn bus_read(&mut self, addr: u32) -> u8;
    fn bus_write(&mut self, addr: u32, data: u8);
}

pub trait Ppu {
    fn ppu_read(&mut self, addr: u32) -> u8;
    fn ppu_write(&mut self, addr: u32, data: u8);
}

pub trait Timing {
    fn elapse(&mut self, clock: u64);
}

pub trait Cartridge {
    fn cartridge_read(&mut self, addr: u32) -> u8;
    fn cartridge_write(&mut self, addr: u32, data: u8);
}
