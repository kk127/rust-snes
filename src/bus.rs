use crate::context;
trait Context: context::Ppu + context::Timing + context::Cartridge {}
impl<T: context::Ppu + context::Timing + context::Cartridge> Context for T {}

pub struct Bus {
    wram: [u8; 0x20000],
}

impl Default for Bus {
    fn default() -> Bus {
        Bus { wram: [0; 0x20000] }
    }
}

impl Bus {
    pub fn read(&mut self, addr: u32, ctx: &mut impl Context) -> u8 {
        let bank = addr >> 16;
        let offset = addr as u16;
        let data = match bank {
            00..=0x3F | 0x80..=0xBF => match offset {
                0x0000..=0x1FFF => self.wram[offset as usize],
                0x2100..=0x213F => ctx.ppu_read(addr),

                0x8000..=0xFFFF => ctx.cartridge_read(addr),
                _ => unimplemented!(),
            },
            0x40..=0x7D => ctx.cartridge_read(addr),
            0x7E..=0x7F => self.wram[(addr & 0x1FFFF) as usize],
            0xC0..=0xFF => ctx.cartridge_read(addr),
            _ => unimplemented!(),
        };
        data
    }

    pub fn write(&mut self, addr: u32, data: u8, ctx: &mut impl Context) {
        let bank = addr >> 16;
        let offset = addr as u16;
        match bank {
            0x00..=0x3F | 0x80..=0xBF => match offset {
                0x0000..=0x1FFF => self.wram[offset as usize] = data,
                0x2100..=0x213F => ctx.ppu_write(addr, data),
                0x8000..=0xFFFF => ctx.cartridge_write(addr, data),
                // _ => unimplemented!(),
                _ => println!("Write unimplemeted, bank: {:x}, offset: {:x}", bank, offset),
            },
            0x40..=0x7D => ctx.cartridge_write(addr, data),
            0x7E..=0x7F => self.wram[(addr & 0x1FFFF) as usize] = data,
            0xC0..=0xFF => ctx.cartridge_write(addr, data),
            _ => unimplemented!(),
        }
    }
}
