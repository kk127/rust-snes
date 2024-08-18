use std::iter::Cycle;

use crate::context;
trait Context: context::Ppu + context::Timing + context::Cartridge {}
impl<T: context::Ppu + context::Timing + context::Cartridge> Context for T {}

const CYCLE_FAST: u64 = 6;
const CYCLE_SLOW: u64 = 8;
const CYCLE_JOYPAD: u64 = 12;

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
                0x0000..=0x1FFF => {
                    ctx.elapse(CYCLE_SLOW);
                    self.wram[offset as usize]
                }
                0x2100..=0x213F => {
                    ctx.elapse(CYCLE_FAST);
                    ctx.ppu_read(addr as u16)
                }

                0x8000..=0xFFFF => {
                    // TODO CYCLE FASTの場合は？
                    ctx.elapse(CYCLE_SLOW);
                    ctx.cartridge_read(addr)
                }
                // TODO
                // _ => unimplemented!("Read unimplemeted, bank: {:x}, offset: {:x}", bank, offset),
                _ => {
                    println!("Read unimplemeted, bank: {:x}, offset: {:x}", bank, offset);
                    0
                }
            },
            0x40..=0x7D => {
                ctx.elapse(CYCLE_SLOW);
                ctx.cartridge_read(addr)
            }
            0x7E..=0x7F => {
                ctx.elapse(CYCLE_SLOW);
                self.wram[(addr & 0x1FFFF) as usize]
            }
            0xC0..=0xFF => {
                // TODO CYCLE FASTの場合は？
                ctx.elapse(CYCLE_SLOW);
                ctx.cartridge_read(addr)
            }
            _ => unimplemented!(),
        };
        data
    }

    pub fn write(&mut self, addr: u32, data: u8, ctx: &mut impl Context) {
        let bank = addr >> 16;
        let offset = addr as u16;
        match bank {
            0x00..=0x3F | 0x80..=0xBF => match offset {
                0x0000..=0x1FFF => {
                    ctx.elapse(CYCLE_SLOW);
                    self.wram[offset as usize] = data;
                }
                0x2100..=0x213F => {
                    ctx.elapse(CYCLE_FAST);
                    ctx.ppu_write(addr as u16, data);
                }
                0x8000..=0xFFFF => {
                    // TODO CYCLE FASTの場合は？
                    ctx.elapse(CYCLE_SLOW);
                    ctx.cartridge_write(addr, data);
                }
                // _ => unimplemented!(),
                _ => {
                    ctx.elapse(CYCLE_SLOW);
                } //println!("Write unimplemeted, bank: {:x}, offset: {:x}", bank, offset),
            },
            0x40..=0x7D => ctx.cartridge_write(addr, data),
            0x7E..=0x7F => self.wram[(addr & 0x1FFFF) as usize] = data,
            0xC0..=0xFF => ctx.cartridge_write(addr, data),
            _ => unimplemented!(),
        }
    }
}
