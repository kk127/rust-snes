use log::{debug, info, warn};
use modular_bitfield::bitfield;
use modular_bitfield::prelude::*;

use crate::controller::Key;
use crate::{context, controller};
trait Context:
    context::Ppu + context::Timing + context::Cartridge + context::Interrupt + context::Spc
{
}
impl<
        T: context::Ppu + context::Timing + context::Cartridge + context::Interrupt + context::Spc,
    > Context for T
{
}

const CYCLE_FAST: u64 = 6;
const CYCLE_SLOW: u64 = 8;
const CYCLE_JOYPAD: u64 = 12;

pub struct Bus {
    wram: [u8; 0x20000],
    wram_addr: u32,
    access_cycle_for_memory2: u64, // 0x420D,

    dma: [Dma; 8],
    gdma_enable: u8,     // 0x420B
    hdma_enable: u8,     // 0x420C
    is_dma_active: bool, // flag for read/write bus in dma (for clock)

    joypad_enable: bool, // 0x4200
    auto_joypad_read_busy: u64,
    controller: [controller::Controller; 2],

    multiplicand: u8,                  // 0x4202
    multiplier: u8,                    // 0x4203
    divident: u16,                     // 0x4204 0x4205
    divisor: u8,                       // 0x4206
    div_result: u16,                   // 0x4214 0x4215
    div_remainder_or_mul_product: u16, // 0x4216 0x4217

    h_count: u16, // 0x4207 0x4208
    v_count: u16, // 0x4209 0x420A

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
            is_dma_active: false,

            controller: Default::default(),
            joypad_enable: false,
            auto_joypad_read_busy: 0,

            multiplicand: 0xFF,
            multiplier: 0xFF,
            divident: 0xFFFF,
            divisor: 0xFF,
            div_result: 0,
            div_remainder_or_mul_product: 0,

            h_count: 0x01FF,
            v_count: 0x01FF,

            open_bus: 0,
        }
    }
}

impl Bus {
    pub fn set_keys(&mut self, keys: [Vec<Key>; 4]) {
        for i in 0..4 {
            let mut data = 0;
            for key in keys[i].iter() {
                match key {
                    Key::B => data |= 1 << 15,
                    Key::Y => data |= 1 << 14,
                    Key::Select => data |= 1 << 13,
                    Key::Start => data |= 1 << 12,
                    Key::Up => data |= 1 << 11,
                    Key::Down => data |= 1 << 10,
                    Key::Left => data |= 1 << 9,
                    Key::Right => data |= 1 << 8,
                    Key::A => data |= 1 << 7,
                    Key::X => data |= 1 << 6,
                    Key::L => data |= 1 << 5,
                    Key::R => data |= 1 << 4,
                }
            }
            self.controller[i % 2].data[i / 2] = data;
        }
    }

    pub fn read(&mut self, addr: u32, ctx: &mut impl Context) -> u8 {
        let bank = addr >> 16;
        let offset = addr as u16;
        let data = match bank {
            00..=0x3F | 0x80..=0xBF => match offset {
                0x0000..=0x1FFF => {
                    if !self.is_dma_active {
                        ctx.elapse(CYCLE_SLOW);
                    }
                    self.wram[offset as usize]
                }
                0x2000..=0x20FF => {
                    if !self.is_dma_active {
                        ctx.elapse(CYCLE_FAST);
                    }
                    warn!(
                        "Read unused region (open_bus): bank: {:X}, offset: {:X}",
                        bank, offset
                    );
                    self.open_bus
                }
                0x2100..=0x213F => {
                    if !self.is_dma_active {
                        ctx.elapse(CYCLE_FAST);
                    }
                    ctx.ppu_read(addr as u16, self.open_bus)
                }
                0x2140..=0x217F => {
                    if !self.is_dma_active {
                        ctx.elapse(CYCLE_FAST);
                    }
                    let port = addr as u16 & 3;
                    let ret = ctx.spc_read(port);
                    debug!("SPC {} -> {:02X} @ {}", addr & 3, ret, ctx.now());
                    ret
                }
                0x2180 => {
                    if !self.is_dma_active {
                        ctx.elapse(CYCLE_FAST);
                    }
                    let data = self.wram[self.wram_addr as usize];
                    self.wram_addr = (self.wram_addr + 1) & 0x1FFFF;
                    data
                }
                0x2181..=0x3FFF => {
                    if !self.is_dma_active {
                        ctx.elapse(CYCLE_FAST);
                    }
                    warn!(
                        "Read unused region (open_bus): bank: {:X}, offset: {:X}",
                        bank, offset
                    );
                    self.open_bus
                }
                0x4000..=0x4015 => {
                    if !self.is_dma_active {
                        ctx.elapse(CYCLE_FAST);
                    }
                    warn!(
                        "Read unused region (open_bus): bank: {:X}, offset: {:X}",
                        bank, offset
                    );
                    self.open_bus
                }
                0x4016 | 0x4017 => {
                    if !self.is_dma_active {
                        ctx.elapse(CYCLE_JOYPAD);
                    }
                    let index = (offset - 0x4016) as usize;
                    // let b0 = self.controller[index as usize].controller_read(4);
                    // let b1 = self.controller[index as usize].controller_read(5);
                    // self.controller[index as usize].controller_write(2, true);
                    // self.controller[index as usize].controller_write(2, false);
                    // TODO open bus
                    // let data = b0 as u8 | (b1 as u8) << 1;

                    let data = self.controller[index].read();
                    if index == 0 {
                        self.open_bus & 0xFC | data
                    } else {
                        self.open_bus & 0xE0 | 0x1C | data
                    }
                }
                0x4018..=0x420F => {
                    if !self.is_dma_active {
                        ctx.elapse(CYCLE_FAST);
                    }
                    warn!(
                        "Read unused region (open_bus): bank: {:X}, offset: {:X}",
                        bank, offset
                    );
                    self.open_bus
                }
                0x4210 => {
                    if !self.is_dma_active {
                        ctx.elapse(CYCLE_FAST);
                    }
                    let nmi_flag = ctx.get_nmi_flag();
                    let cpu_version = 2;
                    (nmi_flag as u8) << 7 | cpu_version | self.open_bus & 0x70
                }

                0x4211 => {
                    // TODO open bus
                    let ret = (ctx.irq_occurred() as u8) << 7;
                    ctx.set_irq(false);
                    ret | self.open_bus & 0x7F
                }

                0x4212 => {
                    if !self.is_dma_active {
                        ctx.elapse(CYCLE_FAST);
                    }
                    let mut ret = 0;
                    ret |= (ctx.now() < self.auto_joypad_read_busy) as u8;
                    ret |= (ctx.is_hblank() as u8) << 6;
                    ret |= (ctx.is_vblank() as u8) << 7;
                    ret | self.open_bus & 0x3E
                }
                0x4213 => {
                    if !self.is_dma_active {
                        ctx.elapse(CYCLE_FAST);
                    }
                    // let b6 = self.controller[0].controller_read(6) as u8;
                    // let b7 = self.controller[1].controller_read(6) as u8;
                    // b6 << 6 | b7 << 7
                    0b1100_0000
                }

                0x4214 => {
                    if !self.is_dma_active {
                        ctx.elapse(CYCLE_FAST);
                    }
                    self.div_result as u8
                }
                0x4215 => {
                    if !self.is_dma_active {
                        ctx.elapse(CYCLE_FAST);
                    }
                    (self.div_result >> 8) as u8
                }
                0x4216 => {
                    if !self.is_dma_active {
                        ctx.elapse(CYCLE_FAST);
                    }
                    self.div_remainder_or_mul_product as u8
                }
                0x4217 => {
                    if !self.is_dma_active {
                        ctx.elapse(CYCLE_FAST);
                    }
                    (self.div_remainder_or_mul_product >> 8) as u8
                }
                0x4218..=0x421F => {
                    if !self.is_dma_active {
                        ctx.elapse(CYCLE_FAST);
                    }
                    let index = (offset as usize - 0x4218) / 2;
                    let pos = (offset as usize - 0x4218) % 2;
                    (self.controller[index % 2].data[index / 2] >> (8 * pos)) as u8
                }
                0x4220..=0x42FF => {
                    if !self.is_dma_active {
                        ctx.elapse(CYCLE_FAST);
                    }
                    warn!(
                        "Read unused region (open_bus): bank: {:X}, offset: {:X}",
                        bank, offset
                    );
                    self.open_bus
                }
                0x4300..=0x437F => {
                    let ch = ((offset >> 4) & 0x7) as usize;
                    let index = offset as u8 & 0xF;
                    self.dma_read(ch, index)
                }
                0x4380..=0x5FFF => {
                    if !self.is_dma_active {
                        ctx.elapse(CYCLE_FAST);
                    }
                    warn!(
                        "Read unused region (open_bus): bank: {:X}, offset: {:X}",
                        bank, offset
                    );
                    self.open_bus
                }
                0x6000..=0xFFFF => {
                    if !self.is_dma_active {
                        // if (0x80..=0xBF).contains(&bank) {
                        //     ctx.elapse(self.access_cycle_for_memory2);
                        // } else {
                        //     ctx.elapse(CYCLE_SLOW);
                        // }
                        if bank & 0x80 == 0 {
                            ctx.elapse(CYCLE_SLOW);
                        } else {
                            ctx.elapse(self.access_cycle_for_memory2);
                        }
                    }

                    ctx.cartridge_read(addr).unwrap_or(self.open_bus)
                }
                // TODO
                // _ => unimplemented!("Read unimplemeted, bank: {:x}, offset: {:x}", bank, offset),
                _ => {
                    debug!("Read unimplemeted, bank: {:x}, offset: {:x}", bank, offset);
                    0
                }
            },
            0x40..=0x7D => {
                if !self.is_dma_active {
                    ctx.elapse(CYCLE_SLOW);
                }
                ctx.cartridge_read(addr).unwrap_or(self.open_bus)
            }
            0x7E..=0x7F => {
                if !self.is_dma_active {
                    ctx.elapse(CYCLE_SLOW);
                }
                self.wram[(addr & 0x1FFFF) as usize]
            }
            0xC0..=0xFF => {
                // TODO CYCLE FASTの場合は？
                if !self.is_dma_active {
                    ctx.elapse(self.access_cycle_for_memory2);
                }
                ctx.cartridge_read(addr).unwrap_or(self.open_bus)
            }
            _ => unimplemented!(),
        };
        self.open_bus = data;
        debug!(
            "Bus read  bank: {:X}, addr: 0x{:X}, data: 0x{:X} ",
            bank, offset, data
        );
        debug!("Bus cpu_open_bus: 0x{:X}", self.open_bus);
        data
    }

    fn dma_read(&mut self, ch: usize, offset: u8) -> u8 {
        match offset {
            0 => self.dma[ch].dma_params.bytes[0],
            1 => self.dma[ch].b_bus_address,
            2 => self.dma[ch].a_bus_address as u8,
            3 => (self.dma[ch].a_bus_address >> 8) as u8,
            4 => self.dma[ch].a_bus_bank,
            5 => self.dma[ch].number_of_bytes_to_transfer as u8,
            6 => (self.dma[ch].number_of_bytes_to_transfer >> 8) as u8,
            7 => self.dma[ch].indirect_hdma_bank,
            8 => self.dma[ch].hdma_table_current_address as u8,
            9 => (self.dma[ch].hdma_table_current_address >> 8) as u8,
            0xA => self.dma[ch].hdma_line_counter,
            0xB | 0xF => self.dma[ch].unused,
            0xC..=0xE => {
                warn!("Invalid DMA read offset: {}", offset);
                self.open_bus
            }
            _ => unreachable!(),
        }
    }

    fn read_16(&mut self, addr: u32, ctx: &mut impl Context) -> u16 {
        let lo = self.read(addr, ctx) as u16;
        let hi = self.read(addr & 0xFF0000 | (addr as u16).wrapping_add(1) as u32, ctx) as u16;
        hi << 8 | lo
    }

    pub fn write(&mut self, addr: u32, data: u8, ctx: &mut impl Context) {
        let bank = addr >> 16;
        let offset = addr as u16;
        self.open_bus = data;
        debug!(
            "Bus write  bank: {:X}, addr: 0x{:X}, data: 0x{:X} ",
            bank, offset, data
        );
        debug!("Bus cpu_open_bus: 0x{:X}", self.open_bus);

        match bank {
            0x00..=0x3F | 0x80..=0xBF => {
                match offset {
                    0x0000..=0x1FFF => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_SLOW);
                        }
                        self.wram[offset as usize] = data;
                    }
                    0x2100..=0x213F => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        ctx.ppu_write(addr as u16, data);
                    }
                    0x2140..=0x217F => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        debug!("SPC {} <- {:02X} @ {}", addr & 3, data, ctx.now());
                        let port = addr as u16 & 3;
                        ctx.spc_write(port, data);
                    }
                    0x2180 => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        self.wram[self.wram_addr as usize] = data;
                        self.wram_addr = (self.wram_addr + 1) & 0x1FFFF;
                    }
                    0x2181 => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        self.wram_addr = (self.wram_addr & 0x1FF00) | data as u32;
                    }
                    0x2182 => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        self.wram_addr = (self.wram_addr & 0x100FF) | ((data as u32) << 8);
                    }
                    0x2183 => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        self.wram_addr = (self.wram_addr & 0x0FFFF) | ((data as u32 & 1) << 16);
                    }
                    0x4016 => {
                        // self.controller[0].controller_write(3, data & 1 != 0);
                        // self.controller[1].controller_write(3, data & 1 != 0);
                        if data & 1 == 1 {
                            self.controller[0].initialize();
                            self.controller[1].initialize();
                        }
                    }
                    0x4200 => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        let joypad_enable = data & 1 == 1;
                        let hv_irq_enable = (data >> 4) & 3;
                        let nmi_enable = (data >> 7) & 1 == 1;

                        self.joypad_enable = joypad_enable;
                        ctx.set_hv_irq_enable(hv_irq_enable);
                        ctx.set_nmi_enable(nmi_enable);
                        debug!("NMITIMEN = joypad_enable: {}, hv_irq_enable: {}, v_blank_nmi_enable: {}", joypad_enable, hv_irq_enable, nmi_enable);
                    }

                    0x4201 => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        debug!("Unimplemented: 0x{:x} = 0x{:x}", addr, data);
                        // self.controller[0].controller_write(6, data & (1 << 6) != 0);
                        // self.controller[1].controller_write(6, data & (1 << 7) != 0);
                    }

                    0x4202 => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        self.multiplicand = data;
                    }
                    0x4203 => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        self.multiplier = data;
                        // TODO Wait 8 clk cycles, then read the 16bit result from Port 4216h-4217h.
                        self.div_remainder_or_mul_product =
                            (self.multiplicand as u16) * (self.multiplier as u16);
                        self.div_result = data as u16;
                    }
                    0x4204 => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        self.divident = (self.divident & 0xFF00) | data as u16;
                    }
                    0x4205 => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        self.divident = ((data as u16) << 8) | (self.divident & 0x00FF);
                    }
                    0x4206 => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        self.divisor = data;
                        // TODO Wait 16 clk cycles, then read the 16bit result from Port 4214h-4215h.
                        if self.divisor == 0 {
                            self.div_result = 0xFFFF;
                            self.div_remainder_or_mul_product = self.divident;
                        } else {
                            self.div_result = self.divident / self.divisor as u16;
                            self.div_remainder_or_mul_product = self.divident % self.divisor as u16;
                        }
                    }
                    0x4207 => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        self.h_count = (self.h_count & 0x0100) | data as u16;
                        ctx.set_h_count(self.h_count);
                    }
                    0x4208 => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        self.h_count = (data as u16) << 8 | (self.h_count & 0x00FF);
                        ctx.set_h_count(self.h_count);
                    }
                    0x4209 => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        self.v_count = (self.v_count & 0x0100) | data as u16;
                        ctx.set_v_count(self.v_count);
                    }
                    0x420A => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        self.v_count = (data as u16) << 8 | (self.v_count & 0x00FF);
                        ctx.set_v_count(self.v_count);
                    }
                    0x420B => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        self.gdma_enable = data;
                        debug!("GDMA Enable: {data:08b} @ y = {}", ctx.counter().y);
                    }
                    0x420C => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        self.hdma_enable = data;
                        // debug!("HDMA enable: 0x{:x}", data);
                        debug!("HDMA Enable: {data:08b} @ y = {}", ctx.counter().y);
                    }
                    0x420D => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        self.access_cycle_for_memory2 = if data & 1 == 1 { 6 } else { 8 };
                    }
                    0x4300..=0x437F => {
                        if !self.is_dma_active {
                            ctx.elapse(CYCLE_FAST);
                        }
                        let ch = ((offset >> 4) & 0xF) as usize;
                        let index = offset as usize & 0xF;
                        self.dma_write(ctx, ch, index, data);
                    }
                    0x6000..=0xFFFF => {
                        if !self.is_dma_active {
                            if (0x80..=0xBF).contains(&bank) {
                                ctx.elapse(self.access_cycle_for_memory2);
                            } else {
                                ctx.elapse(CYCLE_SLOW);
                            }
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
                    }
                }
            }
            0x40..=0x7D => {
                if !self.is_dma_active {
                    ctx.elapse(CYCLE_SLOW);
                }
                ctx.cartridge_write(addr, data);
            }
            0x7E..=0x7F => {
                if !self.is_dma_active {
                    ctx.elapse(CYCLE_SLOW);
                }
                self.wram[(addr & 0x1FFFF) as usize] = data;
                debug!("Write WRAM: {addr:04X} = {data:02X}");
            }
            0xC0..=0xFF => {
                if !self.is_dma_active {
                    ctx.elapse(self.access_cycle_for_memory2);
                }
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
        debug!("ch: {}, index: {}, data: 0x{:x}", ch, index, data);
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
            _ => warn!("Invalid DMA index: {}", index),
        }
    }

    fn gdma_exec(&mut self, ctx: &mut impl Context) {
        if self.gdma_enable == 0 {
            return;
        }

        debug!("gdma_enable: {:08b}", self.gdma_enable);
        debug!("GDMA Exec: start: {}", ctx.now());
        self.is_dma_active = true;
        // ctx.elapse(8 - ctx.now() % 8);
        let ch = self.gdma_enable.trailing_zeros() as usize;
        // ctx.elapse(8);
        let transfer_unit = self.dma[ch].transfer_unit();
        let a_step = match self.dma[ch].dma_params.a_bus_address_step() {
            AbusAddressStep::Increment => 1,
            AbusAddressStep::Fixed1 => 0,
            AbusAddressStep::Decrement => (-1 as i16) as u16,
            AbusAddressStep::Fixed3 => 0,
        };
        for i in 0..transfer_unit.len() {
            ctx.elapse(8);
            let a_bus = (self.dma[ch].a_bus_bank as u32) << 16 | self.dma[ch].a_bus_address as u32;
            let b_bus = 0x2100 | self.dma[ch].b_bus_address.wrapping_add(transfer_unit[i]) as u32;

            match self.dma[ch].dma_params.transfer_direction() {
                TransferDirection::AtoB => {
                    let data = self.read(a_bus, ctx);
                    debug!("interval in read and write: {}", ctx.now());
                    self.write(b_bus, data, ctx);
                    debug!("after write: {}", ctx.now());
                }
                TransferDirection::BtoA => {
                    let data = self.read(b_bus, ctx);
                    debug!("interval in read and write: {}", ctx.now());
                    self.write(a_bus, data, ctx);
                    debug!("after write: {}", ctx.now());
                }
            }
            debug!("now: {}", ctx.now());

            self.dma[ch].a_bus_address = self.dma[ch].a_bus_address.wrapping_add(a_step);
            self.dma[ch].number_of_bytes_to_transfer =
                self.dma[ch].number_of_bytes_to_transfer.wrapping_sub(1);

            if self.dma[ch].number_of_bytes_to_transfer == 0 {
                self.gdma_enable &= !(1 << ch);
                ctx.elapse(16);
                break;
            }
            debug!("a_bus: {:06X}, b_bus: {:06X}", a_bus, b_bus);
        }
        debug!(
            "GDMA[{ch}]: {:02X}:{:04X} {} 21{:02X}, trans: {:?}, count: {}, now: {}",
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
            ctx.now()
        );

        // ctx.elapse(16);
        self.is_dma_active = false;

        debug!("GDMA Exec: end: {}", ctx.now());
    }

    fn hdma_reload_and_exec(&mut self, ctx: &mut impl Context) {
        self.is_dma_active = true;
        if ctx.is_hdma_reload_triggered() {
            debug!(
                "HDMA Reload, frame:x:y = {}:{}:{}, HDMA enable: {:08b}",
                ctx.counter().frame,
                ctx.counter().x,
                ctx.counter().y,
                self.hdma_enable
            );
            for ch in 0..8 {
                self.dma[ch].is_hdma_active = false;
                self.dma[ch].is_hdma_completed = false;
            }
            if self.hdma_enable != 0 {
                ctx.elapse(18);
            }

            for ch in 0..8 {
                if self.hdma_enable >> ch & 1 == 1 {
                    self.hdma_reload(ctx, ch);
                }
            }
        }

        if ctx.is_hdma_transfer_triggered()
            && self.hdma_enable != 0
            && self.dma.iter().any(|d| !d.is_hdma_completed)
        {
            debug!(
                "HDMA Transfer, frame:x:y = {}:{}:{}, now = {}",
                ctx.counter().frame,
                ctx.counter().x,
                ctx.counter().y,
                ctx.now()
            );
            ctx.elapse(18);
            for ch in 0..8 {
                if self.hdma_enable >> ch & 1 == 1 {
                    self.hdma_exec(ctx, ch);
                }
            }
        }
        self.is_dma_active = false;
    }

    fn hdma_reload(&mut self, ctx: &mut impl Context, ch: usize) {
        // TODO Cancel GDMA channle that is using the same channel

        debug!("HDMA{ch} Init: param = {:?}", self.dma[ch].dma_params);
        self.dma[ch].hdma_table_current_address = self.dma[ch].a_bus_address;

        let addr = self.dma[ch].hdma_direct_address(1);
        let data = self.read(addr, ctx);

        if data == 0 {
            info!("HDMA{ch}: Empty table");
            self.dma[ch].is_hdma_completed = true;
            return;
        }
        self.dma[ch].hdma_line_counter = data;

        if self.dma[ch].dma_params.hdma_addr_mode() == HdmaAddrMode::Indirect {
            let addr = self.dma[ch].hdma_direct_address(2);
            let data = self.read_16(addr, ctx);
            self.dma[ch].number_of_bytes_to_transfer = data;
            debug!(
                "HDMA{ch}: Indirect addr = {:04X}",
                self.dma[ch].number_of_bytes_to_transfer
            );
            ctx.elapse(16);
        }

        self.dma[ch].is_hdma_active = true;
        debug!(
            "HDMA {ch} Reload: Line counter: {:02X}, addr: {:04X} -> 21{:02X}",
            self.dma[ch].hdma_line_counter,
            self.dma[ch].hdma_table_current_address,
            self.dma[ch].b_bus_address,
        );
    }

    fn hdma_exec(&mut self, ctx: &mut impl Context, ch: usize) {
        debug!(
            "HDMA {ch}: Begin exec at line {}:{}",
            ctx.counter().y,
            ctx.counter().x
        );
        debug!("HDMA info: {:?}", self.dma[ch]);
        if self.dma[ch].is_hdma_active {
            debug!(
                "HDMA {ch}: Do trans {} bytes",
                self.dma[ch].transfer_unit().len()
            );
            for &offset in self.dma[ch].transfer_unit() {
                let a_bus_addr = match self.dma[ch].dma_params.hdma_addr_mode() {
                    HdmaAddrMode::Direct => self.dma[ch].hdma_direct_address(1),
                    HdmaAddrMode::Indirect => self.dma[ch].hdma_indirect_address(1),
                };
                let b_bus_addr = 0x2100 | self.dma[ch].b_bus_address.wrapping_add(offset) as u32;

                match self.dma[ch].dma_params.transfer_direction() {
                    TransferDirection::AtoB => {
                        let data = self.read(a_bus_addr, ctx);
                        self.write(b_bus_addr, data, ctx);
                        debug!("HDMA: {a_bus_addr:06X} -> {b_bus_addr:04X} = {data:02X}");
                    }
                    TransferDirection::BtoA => {
                        let data = self.read(b_bus_addr, ctx);
                        self.write(a_bus_addr, data, ctx);
                        debug!("HDMA: {b_bus_addr:06X} -> {a_bus_addr:04X} = {data:02X}");
                    }
                }
                ctx.elapse(8);
            }
        }

        self.dma[ch].hdma_line_counter = self.dma[ch].hdma_line_counter.wrapping_sub(1);
        self.dma[ch].is_hdma_active = self.dma[ch].hdma_line_counter & 0x80 != 0;

        debug!(
            "HDMA {ch}: Line counter: {:02X}, Do transfer: {}",
            self.dma[ch].hdma_line_counter, self.dma[ch].is_hdma_active
        );

        if self.dma[ch].hdma_line_counter & 0x7F == 0 {
            let addr = self.dma[ch].hdma_direct_address(1);
            let data = self.read(addr, ctx);
            self.dma[ch].hdma_line_counter = data;

            debug!(
                "HDMA {ch}: New line counter: {:02X}",
                self.dma[ch].hdma_line_counter
            );
            ctx.elapse(8);
            if self.dma[ch].dma_params.hdma_addr_mode() == HdmaAddrMode::Indirect {
                if self.dma[ch].hdma_line_counter != 0 {
                    let addr = self.dma[ch].hdma_direct_address(2);
                    let data = self.read_16(addr, ctx);
                    self.dma[ch].number_of_bytes_to_transfer = data;
                    ctx.elapse(16);
                } else {
                    let addr = self.dma[ch].hdma_direct_address(1);
                    self.dma[ch].number_of_bytes_to_transfer = (self.read(addr, ctx) as u16) << 8;
                    ctx.elapse(8);
                }
            }

            if self.dma[ch].hdma_line_counter == 0 {
                self.dma[ch].is_hdma_completed = true;
                debug!("HDMA {ch}: Done");
            }

            self.dma[ch].is_hdma_active = true;
        }
        debug!(
            "HDMA {ch}: End exec, Do transfer: {}",
            self.dma[ch].is_hdma_active
        );
    }

    fn auto_joypad_read(&mut self) {
        for port in 0..2 {
            // self.controller[port].controller_write(3, true);
            self.controller[port].initialize();
            for _ in 0..16 {
                // self.controller[port].controller_write(2, true);
                // self.controller[port].controller_write(2, false);
                self.controller[port].read();
            }
        }
    }

    pub fn tick(&mut self, ctx: &mut impl Context) {
        if ctx.is_auto_joypad_read() && self.joypad_enable {
            self.auto_joypad_read_busy = ctx.now() + 4224;
            self.auto_joypad_read();
        }
        self.hdma_reload_and_exec(ctx);
        self.gdma_exec(ctx);
    }
}

#[derive(Default, Debug)]
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

    is_hdma_active: bool,
    is_hdma_completed: bool,
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

    fn hdma_direct_address(&mut self, inc: u16) -> u32 {
        let ret = (self.a_bus_bank as u32) << 16 | self.hdma_table_current_address as u32;
        self.hdma_table_current_address = self.hdma_table_current_address.wrapping_add(inc);
        ret
    }

    fn hdma_indirect_address(&mut self, inc: u16) -> u32 {
        // let ret = (self.indirect_hdma_bank as u32) << 16 | self.hdma_table_current_address as u32;
        // self.hdma_table_current_address = self.hdma_table_current_address.wrapping_add(inc);
        // ret
        let ret = (self.indirect_hdma_bank as u32) << 16 | self.number_of_bytes_to_transfer as u32;
        self.number_of_bytes_to_transfer = self.number_of_bytes_to_transfer.wrapping_add(inc);
        ret
    }
}

#[bitfield(bits = 8)]
#[derive(Default, Debug)]
struct DmaParams {
    transfer_unit: B3,
    a_bus_address_step: AbusAddressStep,
    __: B1,
    hdma_addr_mode: HdmaAddrMode,
    transfer_direction: TransferDirection,
}

#[derive(BitfieldSpecifier, Debug)]
#[bits = 2]
enum AbusAddressStep {
    Increment = 0,
    Fixed1 = 1,
    Decrement = 2,
    Fixed3 = 3,
}

#[derive(BitfieldSpecifier, Default, PartialEq, Debug)]
#[bits = 1]
enum HdmaAddrMode {
    #[default]
    Direct = 0,
    Indirect = 1,
}

#[derive(BitfieldSpecifier, Debug)]
#[bits = 1]
enum TransferDirection {
    AtoB = 0,
    BtoA = 1,
}
