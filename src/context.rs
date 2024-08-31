use crate::{bus, cartridge, counter, cpu, interrupt, ppu};

// struct Context {
//     cpu: cpu::Cpu,
//     bus: bus::Bus,
//     ppu: ppu::Ppu,
//     timing: counter::Counter,
//     cartridge: cartridge::Cartridge,
// }

pub struct Context {
    cpu: cpu::Cpu,
    pub inner1: Inner1,
}

pub struct Inner1 {
    bus: bus::Bus,
    pub inner2: Inner2,
}

pub struct Inner2 {
    pub ppu: ppu::Ppu,
    cartridge: cartridge::Cartridge,
    pub inner: Inner3,
}
struct Inner3 {
    timing: counter::Counter,
    interrupt: interrupt::Interrupt,
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
                        interrupt: interrupt::Interrupt::default(),
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

    fn bus_tick(&mut self) {
        self.bus.tick(&mut self.inner2);
    }
}

impl Timing for Inner1 {
    fn elapse(&mut self, clock: u64) {
        self.inner2.elapse(clock)
    }

    fn now(&self) -> u64 {
        self.inner2.now()
    }
}

impl Interrupt for Inner1 {
    fn get_nmi_flag(&mut self) -> bool {
        self.inner2.get_nmi_flag()
    }

    fn set_nmi_flag(&mut self, flag: bool) {
        self.inner2.set_nmi_flag(flag)
    }

    fn nmi_occurred(&self) -> bool {
        self.inner2.nmi_occurred()
    }

    fn set_nmi_enable(&mut self, flag: bool) {
        self.inner2.set_nmi_enable(flag)
    }

    fn set_hv_irq_enable(&mut self, val: u8) {
        self.inner2.set_hv_irq_enable(val)
    }

    fn get_hv_irq_enable(&self) -> u8 {
        self.inner2.get_hv_irq_enable()
    }

    fn set_joypad_enable(&mut self, flag: bool) {
        self.inner2.set_joypad_enable(flag)
    }
}

impl Ppu for Inner2 {
    fn ppu_read(&mut self, addr: u16) -> u8 {
        self.ppu.read(addr, &mut self.inner)
    }

    fn ppu_write(&mut self, addr: u16, data: u8) {
        self.ppu.write(addr, data, &mut self.inner)
    }

    fn ppu_tick(&mut self) {
        self.ppu.tick(&mut self.inner)
    }

    fn is_hblank(&self) -> bool {
        self.ppu.is_hblank()
    }

    fn is_vblank(&self) -> bool {
        self.ppu.is_vblank()
    }

    fn is_hdma_reload_triggered(&mut self) -> bool {
        self.ppu.is_hdma_reload_triggered()
    }
    fn is_hdma_transfer_triggered(&mut self) -> bool {
        self.ppu.is_hdma_transfer_triggered()
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
    fn now(&self) -> u64 {
        self.inner.timing.now()
    }
}

impl Interrupt for Inner2 {
    fn get_nmi_flag(&mut self) -> bool {
        self.inner.interrupt.get_nmi_flag()
    }

    fn set_nmi_flag(&mut self, flag: bool) {
        self.inner.interrupt.set_nmi_flag(flag)
    }

    fn nmi_occurred(&self) -> bool {
        self.inner.interrupt.nmi_occurred()
    }

    fn get_hv_irq_enable(&self) -> u8 {
        self.inner.interrupt.get_hv_irq_enable()
    }

    fn set_nmi_enable(&mut self, flag: bool) {
        self.inner.interrupt.set_nmi_enable(flag)
    }

    fn set_hv_irq_enable(&mut self, val: u8) {
        self.inner.interrupt.set_hv_irq_enable(val)
    }

    fn set_joypad_enable(&mut self, flag: bool) {
        self.inner.interrupt.set_joypad_enable(flag)
    }
}

impl Timing for Inner3 {
    fn elapse(&mut self, clock: u64) {
        self.timing.elapse(clock)
    }
    fn now(&self) -> u64 {
        self.timing.now()
    }
}

impl Interrupt for Inner3 {
    fn get_nmi_flag(&mut self) -> bool {
        self.interrupt.get_nmi_flag()
    }

    fn set_nmi_flag(&mut self, flag: bool) {
        self.interrupt.set_nmi_flag(flag)
    }

    fn nmi_occurred(&self) -> bool {
        self.interrupt.nmi_occurred()
    }

    fn set_nmi_enable(&mut self, flag: bool) {
        self.interrupt.set_nmi_enable(flag)
    }

    fn set_hv_irq_enable(&mut self, val: u8) {
        self.interrupt.set_hv_irq_enable(val)
    }

    fn get_hv_irq_enable(&self) -> u8 {
        self.interrupt.get_hv_irq_enable()
    }

    fn set_joypad_enable(&mut self, flag: bool) {
        self.interrupt.set_joypad_enable(flag)
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

    fn bus_tick(&mut self);
}

pub trait Ppu {
    fn ppu_read(&mut self, addr: u16) -> u8;
    fn ppu_write(&mut self, addr: u16, data: u8);

    fn ppu_tick(&mut self);

    fn is_hblank(&self) -> bool;
    fn is_vblank(&self) -> bool;
    fn is_hdma_reload_triggered(&mut self) -> bool;
    fn is_hdma_transfer_triggered(&mut self) -> bool;
}

pub trait Timing {
    fn elapse(&mut self, clock: u64);
    fn now(&self) -> u64;
}

pub trait Cartridge {
    fn cartridge_read(&mut self, addr: u32) -> u8;
    fn cartridge_write(&mut self, addr: u32, data: u8);
}

pub trait Interrupt {
    fn get_nmi_flag(&mut self) -> bool;
    fn set_nmi_flag(&mut self, flag: bool);
    fn nmi_occurred(&self) -> bool;
    fn set_nmi_enable(&mut self, flag: bool);
    fn set_hv_irq_enable(&mut self, val: u8);
    fn get_hv_irq_enable(&self) -> u8;
    fn set_joypad_enable(&mut self, flag: bool);
}
