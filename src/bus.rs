use log::{debug, info};
use modular_bitfield::bitfield;
use modular_bitfield::prelude::*;

use crate::context;
trait Context: context::Ppu + context::Timing + context::Cartridge + context::Interrupt {}
impl<T: context::Ppu + context::Timing + context::Cartridge + context::Interrupt> Context for T {}

const CYCLE_FAST: u64 = 6;
const CYCLE_SLOW: u64 = 8;
const CYCLE_JOYPAD: u64 = 12;

pub struct Bus {
    wram: [u8; 0x20000],
    wram_addr: u32,
    access_cycle_for_memory2: u64, // 0x420D,

    dma: [Dma; 8],
    gdma_enable: u8, // 0x420B
    hdma_enable: u8, // 0x420C

    open_bus: u8,
}

impl Default for Bus {
    fn default() -> Bus {
        Bus {
            wram: [0; 0x20000],
            wram_addr: 0,
            access_cycle_for_memory2: 8,

            dma: Default::default(),
            gdma_enable: 0,
            hdma_enable: 0,

            open_bus: 0,
        }
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

                0x4210 => {
                    let nmi_flag = ctx.get_nmi_flag();
                    let cpu_version = 2;
                    println!(
                        "nmi_flag << 7 | cpu_version = {}",
                        (nmi_flag as u8) << 7 | cpu_version
                    );
                    println!("open_bus = {}", self.open_bus);
                    (nmi_flag as u8) << 7 | cpu_version | self.open_bus
                }

                0x8000..=0xFFFF => {
                    if (0x80..=0xBF).contains(&bank) {
                        ctx.elapse(self.access_cycle_for_memory2);
                    } else {
                        ctx.elapse(CYCLE_SLOW);
                    }
                    ctx.cartridge_read(addr)
                }
                // TODO
                // _ => unimplemented!("Read unimplemeted, bank: {:x}, offset: {:x}", bank, offset),
                _ => {
                    debug!("Read unimplemeted, bank: {:x}, offset: {:x}", bank, offset);
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
        self.open_bus = data;
        data
    }

    pub fn write(&mut self, addr: u32, data: u8, ctx: &mut impl Context) {
        let bank = addr >> 16;
        let offset = addr as u16;
        self.open_bus = data;
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
                0x2180 => {
                    ctx.elapse(CYCLE_FAST);
                    self.wram[self.wram_addr as usize] = data;
                    self.wram_addr = (self.wram_addr + 1) & 0x1FFFF;
                }
                0x2181 => {
                    ctx.elapse(CYCLE_FAST);
                    self.wram_addr = (self.wram_addr & 0x1FF00) | data as u32;
                }
                0x2182 => {
                    ctx.elapse(CYCLE_FAST);
                    self.wram_addr = (self.wram_addr & 0x100FF) | ((data as u32) << 8);
                }
                0x2183 => {
                    ctx.elapse(CYCLE_FAST);
                    self.wram_addr = (self.wram_addr & 0x0FFFF) | ((data as u32 & 1) << 16);
                }
                0x4016 | 0x4017 => {
                    debug!("write to joypad register: 0x{:x} = 0x{:x}", addr, data);
                    debug!("Unimplemented");
                }
                0x4200 => {
                    ctx.elapse(CYCLE_FAST);
                    let joypad_enable = data & 1 == 1;
                    let hv_irq_enable = (data >> 4) & 3;
                    let nmi_enable = (data >> 7) & 1 == 1;

                    ctx.set_joypad_enable(joypad_enable);
                    ctx.set_hv_irq_enable(hv_irq_enable);
                    ctx.set_nmi_enable(nmi_enable);
                }
                0x420B => {
                    ctx.elapse(CYCLE_FAST);
                    self.gdma_enable = data;
                }
                0x420C => {
                    ctx.elapse(CYCLE_FAST);
                    self.hdma_enable = data;
                }
                0x420D => {
                    ctx.elapse(CYCLE_FAST);
                    self.access_cycle_for_memory2 = if data & 1 == 1 { 6 } else { 8 };
                }
                0x4300..=0x437F => {
                    let ch = ((offset >> 4) & 0xF) as usize;
                    let index = offset as usize & 0xF;
                    self.dma_write(ctx, ch, index, data);
                }
                0x8000..=0xFFFF => {
                    if (0x80..=0xBF).contains(&bank) {
                        ctx.elapse(self.access_cycle_for_memory2);
                    } else {
                        ctx.elapse(CYCLE_SLOW);
                    }
                    ctx.cartridge_write(addr, data);
                }
                // _ => unimplemented!(),
                _ => {
                    ctx.elapse(CYCLE_SLOW);
                    debug!(
                        "Write unimplemeted, bank: 0x{:x}, offset: 0x{:x} = data: 0x{0:x}",
                        bank, offset
                    );
                } //println!("Write unimplemeted, bank: {:x}, offset: {:x}", bank, offset),
            },
            0x40..=0x7D => {
                ctx.elapse(CYCLE_SLOW);
                ctx.cartridge_write(addr, data);
            }
            0x7E..=0x7F => {
                ctx.elapse(CYCLE_SLOW);
                self.wram[(addr & 0x1FFFF) as usize] = data;
            }
            0xC0..=0xFF => {
                ctx.elapse(self.access_cycle_for_memory2);
                ctx.cartridge_write(addr, data);
            }
            // _ => unimplemented!(),
            _ => debug!(
                "Write unimplemeted, bank: 0x{:x}, offset: 0x{:x} = data: 0x{:x}",
                bank, offset, data
            ),
        }
    }

    fn dma_write(&mut self, ctx: &mut impl Context, ch: usize, index: usize, data: u8) {
        match index {
            0 => self.dma[ch].dma_params.bytes[0] = data,
            1 => self.dma[ch].b_bus_address = data,
            2 => self.dma[ch].a_bus_address = (self.dma[ch].a_bus_address & 0xFF00) | data as u16,
            3 => {
                self.dma[ch].a_bus_address =
                    (self.dma[ch].a_bus_address & 0x00FF) | (data as u16) << 8
            }
            4 => self.dma[ch].a_bus_bank = data,
            5 => {
                self.dma[ch].number_of_bytes_to_transfer =
                    (self.dma[ch].number_of_bytes_to_transfer & 0xFF00) | data as u16
            }
            6 => {
                self.dma[ch].number_of_bytes_to_transfer =
                    (self.dma[ch].number_of_bytes_to_transfer & 0x00FF) | (data as u16) << 8
            }
            7 => self.dma[ch].indirect_hdma_bank = data,
            8 => {
                self.dma[ch].hdma_table_current_address =
                    (self.dma[ch].hdma_table_current_address & 0xFF00) | data as u16
            }
            9 => {
                self.dma[ch].hdma_table_current_address =
                    (self.dma[ch].hdma_table_current_address & 0x00FF) | (data as u16) << 8
            }
            0xa => self.dma[ch].hdma_line_counter = data,
            0xb => self.dma[ch].unused = data,
            _ => unreachable!(),
        }
    }

    fn gdma_exec(&mut self, ctx: &mut impl Context) {
        for ch in 0..8 {
            if self.gdma_enable >> ch & 1 == 1 {
                let transfer_unit = self.dma[ch].transfer_unit();
                let mut flag = false;
                let a_step = match self.dma[ch].dma_params.a_bus_address_step() {
                    AbusAddressStep::Increment => 1,
                    AbusAddressStep::Fixed1 => 0,
                    AbusAddressStep::Decrement => (-1 as i16) as u16,
                    AbusAddressStep::Fixed3 => 0,
                };
                debug!(
                    "GDMA[{ch}]: {:02X}:{:04X} {} 21{:02X}, trans: {:?}, count: {}",
                    self.dma[ch].a_bus_bank,
                    self.dma[ch].a_bus_address,
                    if matches!(
                        self.dma[ch].dma_params.transfer_direction(),
                        TransferDirection::AtoB
                    ) {
                        "->"
                    } else {
                        "<-"
                    },
                    self.dma[ch].b_bus_address,
                    transfer_unit,
                    self.dma[ch].number_of_bytes_to_transfer,
                );
                loop {
                    for i in 0..transfer_unit.len() {
                        let a_bus = (self.dma[ch].a_bus_bank as u32) << 16
                            | self.dma[ch].a_bus_address as u32;
                        let b_bus = 0x2100
                            | self.dma[ch].b_bus_address.wrapping_add(transfer_unit[i]) as u32;

                        match self.dma[ch].dma_params.transfer_direction() {
                            TransferDirection::AtoB => {
                                let data = self.read(a_bus, ctx);
                                self.write(b_bus, data, ctx);
                            }
                            TransferDirection::BtoA => {
                                let data = self.read(b_bus, ctx);
                                self.write(a_bus, data, ctx);
                            }
                        }

                        self.dma[ch].a_bus_address =
                            self.dma[ch].a_bus_address.wrapping_add(a_step);
                        self.dma[ch].number_of_bytes_to_transfer =
                            self.dma[ch].number_of_bytes_to_transfer.wrapping_sub(1);

                        if self.dma[ch].number_of_bytes_to_transfer == 0 {
                            flag = true;
                            break;
                        }
                    }
                    if flag {
                        break;
                    }
                }

                self.gdma_enable &= !(1 << ch);
            }
        }
    }

    pub fn tick(&mut self, ctx: &mut impl Context) {
        self.gdma_exec(ctx);
    }
}

#[derive(Default)]
struct Dma {
    dma_params: DmaParams,            // 0x43x0
    b_bus_address: u8,                // 0x43x1
    a_bus_address: u16,               // 0x43x2 0x43x3
    a_bus_bank: u8,                   // 0x43x4
    number_of_bytes_to_transfer: u16, // 0x43x5 0x43x6
    indirect_hdma_bank: u8,           // 0x43x7
    hdma_table_current_address: u16,  // 0x43x8 0x43x9
    hdma_line_counter: u8,            // 0x43xA
    unused: u8,                       // 0x43xB
}

impl Dma {
    fn transfer_unit(&self) -> &'static [u8] {
        match self.dma_params.transfer_unit() {
            0 => &[0],
            1 => &[0, 1],
            2 | 6 => &[0, 0],
            3 | 7 => &[0, 0, 1, 1],
            4 => &[0, 1, 2, 3],
            5 => &[0, 1, 0, 1],
            _ => unreachable!(),
        }
    }
}

#[bitfield(bits = 8)]
#[derive(Default)]
struct DmaParams {
    transfer_unit: B3,
    a_bus_address_step: AbusAddressStep,
    __: B1,
    hdma_address_mode: B1,
    transfer_direction: TransferDirection,
}

#[derive(BitfieldSpecifier)]
#[bits = 2]
enum AbusAddressStep {
    Increment = 0,
    Fixed1 = 1,
    Decrement = 2,
    Fixed3 = 3,
}

#[derive(BitfieldSpecifier)]
#[bits = 1]
enum TransferDirection {
    AtoB = 0,
    BtoA = 1,
}
