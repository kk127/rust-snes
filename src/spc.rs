use log::{debug, warn};
use modular_bitfield::bitfield;

use crate::context;
use crate::dsp;

trait Context: context::Timing {}
impl<T: context::Timing> Context for T {}

#[derive(Default)]
pub struct Spc {
    registers: Registers,
    pub io_registers: IORegisters,
    counter: u64,
    prev_counter: u64,
    dsp_counter: u64,

    sleep: bool,
    stop: bool,

    // for debug
    instruction_counter: u64,
}

const ROM: [u8; 0x40] = [
    0xCD, 0xEF, 0xBD, 0xE8, 0x00, 0xC6, 0x1D, 0xD0, 0xFC, 0x8F, 0xAA, 0xF4, 0x8F, 0xBB, 0xF5, 0x78,
    0xCC, 0xF4, 0xD0, 0xFB, 0x2F, 0x19, 0xEB, 0xF4, 0xD0, 0xFC, 0x7E, 0xF4, 0xD0, 0x0B, 0xE4, 0xF5,
    0xCB, 0xF4, 0xD7, 0x00, 0xFC, 0xD0, 0xF3, 0xAB, 0x01, 0x10, 0xEF, 0x7E, 0xF4, 0x10, 0xEB, 0xBA,
    0xF6, 0xDA, 0x00, 0xBA, 0xF4, 0xC4, 0xF4, 0xDD, 0x5D, 0xD0, 0xDB, 0x1F, 0x00, 0x00, 0xC0, 0xFF,
];

impl Spc {
    pub fn tick(&mut self, ctx: &mut impl Context) {
        let clock_from_master = ctx.now() * 102400 / 2147727;

        while self.counter < clock_from_master {
            self.execute_instruction();
        }

        let elapsed = self.counter - self.prev_counter;
        self.prev_counter = self.counter;
        self.io_registers.tick_timer(elapsed);

        self.dsp_counter += elapsed;
        while self.dsp_counter >= 32 {
            self.dsp_counter -= 32;
            self.io_registers.dsp.tick();
        }
    }

    pub fn audio_buffer(&self) -> &[(i16, i16)] {
        self.io_registers.dsp.get_audio_buffer()
    }

    pub fn clear_audio_buffer(&mut self) {
        self.io_registers.dsp.clear_audio_buffer();
    }

    pub fn write_port(&mut self, port: u16, data: u8) {
        self.io_registers.cpu_in[port as usize] = data;
    }

    pub fn read_port(&mut self, port: u16) -> u8 {
        self.io_registers.cpu_out[port as usize]
    }

    fn increment_counter(&mut self, count: u64) {
        self.counter += count;
    }

    fn execute_instruction(&mut self) {
        let pc = self.registers.pc;
        let op = self.fetch_8();
        match op {
            0x00 => self.nop(),
            0x01 => self.tcall_n(0),
            0x02 => self.set_n_bit(0),
            0x03 => self.bb_sc(0, true),
            0x04 => self.alu(AluType::Or, AddressingMode::DirectPage),
            0x05 => self.alu(AluType::Or, AddressingMode::Absolute),
            0x06 => self.alu(AluType::Or, AddressingMode::IndirectX),
            0x07 => self.alu(AluType::Or, AddressingMode::XIndexedIndirect),
            0x08 => self.alu(AluType::Or, AddressingMode::Immediate),
            0x09 => self.alu(AluType::Or, AddressingMode::DirectPageToDirectPage),
            0x0A => self.or1_c_aaa_b(),
            0x0B => self.asl_with_addressing(AddressingMode::DirectPage),
            0x0C => self.asl_with_addressing(AddressingMode::Absolute),
            0x0D => self.php(),
            0x0E => self.tset(),
            0x0F => self.brk(),

            0x10 => self.br(BranchType::Bpl),
            0x11 => self.tcall_n(1),
            0x12 => self.clr_n_bit(0),
            0x13 => self.bb_sc(0, false),
            0x14 => self.alu(AluType::Or, AddressingMode::XIndexedDirectPage),
            0x15 => self.alu(AluType::Or, AddressingMode::XIndexedAbsolute),
            0x16 => self.alu(AluType::Or, AddressingMode::YIndexedAbsolute),
            0x17 => self.alu(AluType::Or, AddressingMode::IndirectYIndexedIndirect),
            0x18 => self.alu(AluType::Or, AddressingMode::ImmediateDataToDirectPage),
            0x19 => self.alu(AluType::Or, AddressingMode::IndirectPageToIndirectPage),
            0x1A => self.decw(),
            0x1B => self.asl_with_addressing(AddressingMode::XIndexedDirectPage),
            0x1C => self.asl_a(),
            0x1D => self.dec_reg(Register::X),
            0x1E => self.cmp_x(AddressingMode::Absolute),
            0x1F => self.jmp_x_abs(),

            0x20 => self.clrp(),
            0x21 => self.tcall_n(2),
            0x22 => self.set_n_bit(1),
            0x23 => self.bb_sc(1, true),
            0x24 => self.alu(AluType::And, AddressingMode::DirectPage),
            0x25 => self.alu(AluType::And, AddressingMode::Absolute),
            0x26 => self.alu(AluType::And, AddressingMode::IndirectX),
            0x27 => self.alu(AluType::And, AddressingMode::XIndexedIndirect),
            0x28 => self.alu(AluType::And, AddressingMode::Immediate),
            0x29 => self.alu(AluType::And, AddressingMode::DirectPageToDirectPage),
            0x2A => self.or_not1_c_aaa_b(),
            0x2B => self.rol_with_addressing(AddressingMode::DirectPage),
            0x2C => self.rol_with_addressing(AddressingMode::Absolute),
            0x2D => self.pha(),
            0x2E => self.cbne(AddressingMode::DirectPage),
            0x2F => self.br(BranchType::Bra),

            0x30 => self.br(BranchType::Bmi),
            0x31 => self.tcall_n(3),
            0x32 => self.clr_n_bit(1),
            0x33 => self.bb_sc(1, false),
            0x34 => self.alu(AluType::And, AddressingMode::XIndexedDirectPage),
            0x35 => self.alu(AluType::And, AddressingMode::XIndexedAbsolute),
            0x36 => self.alu(AluType::And, AddressingMode::YIndexedAbsolute),
            0x37 => self.alu(AluType::And, AddressingMode::IndirectYIndexedIndirect),
            0x38 => self.alu(AluType::And, AddressingMode::ImmediateDataToDirectPage),
            0x39 => self.alu(AluType::And, AddressingMode::IndirectPageToIndirectPage),
            0x3A => self.incw(),
            0x3B => self.rol_with_addressing(AddressingMode::XIndexedDirectPage),
            0x3C => self.rol_a(),
            0x3D => self.inc_reg(Register::X),
            0x3E => self.cmp_x(AddressingMode::DirectPage),
            0x3F => self.call(),

            0x40 => self.setp(),
            0x41 => self.tcall_n(4),
            0x42 => self.set_n_bit(2),
            0x43 => self.bb_sc(2, true),
            0x44 => self.alu(AluType::Eor, AddressingMode::DirectPage),
            0x45 => self.alu(AluType::Eor, AddressingMode::Absolute),
            0x46 => self.alu(AluType::Eor, AddressingMode::IndirectX),
            0x47 => self.alu(AluType::Eor, AddressingMode::XIndexedIndirect),
            0x48 => self.alu(AluType::Eor, AddressingMode::Immediate),
            0x49 => self.alu(AluType::Eor, AddressingMode::DirectPageToDirectPage),
            0x4A => self.and1_c_aaa_b(),
            0x4B => self.lsr_with_addressing(AddressingMode::DirectPage),
            0x4C => self.lsr_with_addressing(AddressingMode::Absolute),
            0x4D => self.phx(),
            0x4E => self.tclr(),
            0x4F => self.pcall(),

            0x50 => self.br(BranchType::Bvc),
            0x51 => self.tcall_n(5),
            0x52 => self.clr_n_bit(2),
            0x53 => self.bb_sc(2, false),
            0x54 => self.alu(AluType::Eor, AddressingMode::XIndexedDirectPage),
            0x55 => self.alu(AluType::Eor, AddressingMode::XIndexedAbsolute),
            0x56 => self.alu(AluType::Eor, AddressingMode::YIndexedAbsolute),
            0x57 => self.alu(AluType::Eor, AddressingMode::IndirectYIndexedIndirect),
            0x58 => self.alu(AluType::Eor, AddressingMode::ImmediateDataToDirectPage),
            0x59 => self.alu(AluType::Eor, AddressingMode::IndirectPageToIndirectPage),
            0x5A => self.cmpw(),
            0x5B => self.lsr_with_addressing(AddressingMode::XIndexedDirectPage),
            0x5C => self.lsr_a(),
            0x5D => self.tax(),
            0x5E => self.cmp_y(AddressingMode::Absolute),
            0x5F => self.jmp_abs(),

            0x60 => self.clr_c(),
            0x61 => self.tcall_n(6),
            0x62 => self.set_n_bit(3),
            0x63 => self.bb_sc(3, true),
            0x64 => self.alu(AluType::Cmp, AddressingMode::DirectPage),
            0x65 => self.alu(AluType::Cmp, AddressingMode::Absolute),
            0x66 => self.alu(AluType::Cmp, AddressingMode::IndirectX),
            0x67 => self.alu(AluType::Cmp, AddressingMode::XIndexedIndirect),
            0x68 => self.alu(AluType::Cmp, AddressingMode::Immediate),
            0x69 => self.alu(AluType::Cmp, AddressingMode::DirectPageToDirectPage),
            0x6A => self.and_not1_c_aaa_b(),
            0x6B => self.ror_with_addressing(AddressingMode::DirectPage),
            0x6C => self.ror_with_addressing(AddressingMode::Absolute),
            0x6D => self.phy(),
            0x6E => self.dbnz_dp(),
            0x6F => self.ret(),

            0x70 => self.br(BranchType::Bvs),
            0x71 => self.tcall_n(7),
            0x72 => self.clr_n_bit(3),
            0x73 => self.bb_sc(3, false),
            0x74 => self.alu(AluType::Cmp, AddressingMode::XIndexedDirectPage),
            0x75 => self.alu(AluType::Cmp, AddressingMode::XIndexedAbsolute),
            0x76 => self.alu(AluType::Cmp, AddressingMode::YIndexedAbsolute),
            0x77 => self.alu(AluType::Cmp, AddressingMode::IndirectYIndexedIndirect),
            0x78 => self.alu(AluType::Cmp, AddressingMode::ImmediateDataToDirectPage),
            0x79 => self.alu(AluType::Cmp, AddressingMode::IndirectPageToIndirectPage),
            0x7A => self.addw(),
            0x7B => self.ror_with_addressing(AddressingMode::XIndexedDirectPage),
            0x7C => self.ror_a(),
            0x7D => self.txa(),
            0x7E => self.cmp_y(AddressingMode::DirectPage),
            0x7F => self.reti(),

            0x80 => self.set_c(),
            0x81 => self.tcall_n(8),
            0x82 => self.set_n_bit(4),
            0x83 => self.bb_sc(4, true),
            0x84 => self.alu(AluType::Adc, AddressingMode::DirectPage),
            0x85 => self.alu(AluType::Adc, AddressingMode::Absolute),
            0x86 => self.alu(AluType::Adc, AddressingMode::IndirectX),
            0x87 => self.alu(AluType::Adc, AddressingMode::XIndexedIndirect),
            0x88 => self.alu(AluType::Adc, AddressingMode::Immediate),
            0x89 => self.alu(AluType::Adc, AddressingMode::DirectPageToDirectPage),
            0x8A => self.eor1_c_aaa_b(),
            0x8B => self.dec_with_addressing(AddressingMode::DirectPage),
            0x8C => self.dec_with_addressing(AddressingMode::Absolute),
            0x8D => self.ldy(AddressingMode::Immediate),
            0x8E => self.plp(),
            0x8F => self.mov_dp_imm(),

            0x90 => self.br(BranchType::Bcc),
            0x91 => self.tcall_n(9),
            0x92 => self.clr_n_bit(4),
            0x93 => self.bb_sc(4, false),
            0x94 => self.alu(AluType::Adc, AddressingMode::XIndexedDirectPage),
            0x95 => self.alu(AluType::Adc, AddressingMode::XIndexedAbsolute),
            0x96 => self.alu(AluType::Adc, AddressingMode::YIndexedAbsolute),
            0x97 => self.alu(AluType::Adc, AddressingMode::IndirectYIndexedIndirect),
            0x98 => self.alu(AluType::Adc, AddressingMode::ImmediateDataToDirectPage),
            0x99 => self.alu(AluType::Adc, AddressingMode::IndirectPageToIndirectPage),
            0x9A => self.subw(),
            0x9B => self.dec_with_addressing(AddressingMode::XIndexedDirectPage),
            0x9C => self.dec_reg(Register::A),
            0x9D => self.tsx(),
            0x9E => self.div(),
            0x9F => self.xcn(),

            0xA0 => self.ei(),
            0xA1 => self.tcall_n(10),
            0xA2 => self.set_n_bit(5),
            0xA3 => self.bb_sc(5, true),
            0xA4 => self.alu(AluType::Sbc, AddressingMode::DirectPage),
            0xA5 => self.alu(AluType::Sbc, AddressingMode::Absolute),
            0xA6 => self.alu(AluType::Sbc, AddressingMode::IndirectX),
            0xA7 => self.alu(AluType::Sbc, AddressingMode::XIndexedIndirect),
            0xA8 => self.alu(AluType::Sbc, AddressingMode::Immediate),
            0xA9 => self.alu(AluType::Sbc, AddressingMode::DirectPageToDirectPage),
            0xAA => self.mov1_c_aaa_b(),
            0xAB => self.inc_with_addressing(AddressingMode::DirectPage),
            0xAC => self.inc_with_addressing(AddressingMode::Absolute),
            0xAD => self.cmp_y(AddressingMode::Immediate),
            0xAE => self.pla(),
            0xAF => self.sta(AddressingMode::IndirectAutoIncrement),

            0xB0 => self.br(BranchType::Bcs),
            0xB1 => self.tcall_n(11),
            0xB2 => self.clr_n_bit(5),
            0xB3 => self.bb_sc(5, false),
            0xB4 => self.alu(AluType::Sbc, AddressingMode::XIndexedDirectPage),
            0xB5 => self.alu(AluType::Sbc, AddressingMode::XIndexedAbsolute),
            0xB6 => self.alu(AluType::Sbc, AddressingMode::YIndexedAbsolute),
            0xB7 => self.alu(AluType::Sbc, AddressingMode::IndirectYIndexedIndirect),
            0xB8 => self.alu(AluType::Sbc, AddressingMode::ImmediateDataToDirectPage),
            0xB9 => self.alu(AluType::Sbc, AddressingMode::IndirectPageToIndirectPage),
            0xBA => self.movw_ya_dp(),
            0xBB => self.inc_with_addressing(AddressingMode::XIndexedDirectPage),
            0xBC => self.inc_reg(Register::A),
            0xBD => self.txs(),
            0xBE => self.das(),
            0xBF => self.lda(AddressingMode::IndirectAutoIncrement),

            0xC0 => self.di(),
            0xC1 => self.tcall_n(12),
            0xC2 => self.set_n_bit(6),
            0xC3 => self.bb_sc(6, true),
            0xC4 => self.sta(AddressingMode::DirectPage),
            0xC5 => self.sta(AddressingMode::Absolute),
            0xC6 => self.sta(AddressingMode::IndirectX),
            0xC7 => self.sta(AddressingMode::XIndexedIndirect),
            0xC8 => self.cmp_x(AddressingMode::Immediate),
            0xC9 => self.stx(AddressingMode::Absolute),
            0xCA => self.mov1_aaa_b_c(),
            0xCB => self.sty(AddressingMode::DirectPage),
            0xCC => self.sty(AddressingMode::Absolute),
            0xCD => self.ldx(AddressingMode::Immediate),
            0xCE => self.plx(),
            0xCF => self.mul(),

            0xD0 => self.br(BranchType::Bne),
            0xD1 => self.tcall_n(13),
            0xD2 => self.clr_n_bit(6),
            0xD3 => self.bb_sc(6, false),
            0xD4 => self.sta(AddressingMode::XIndexedDirectPage),
            0xD5 => self.sta(AddressingMode::XIndexedAbsolute),
            0xD6 => self.sta(AddressingMode::YIndexedAbsolute),
            0xD7 => self.sta(AddressingMode::IndirectYIndexedIndirect),
            0xD8 => self.stx(AddressingMode::DirectPage),
            0xD9 => self.stx(AddressingMode::YIndexedDirectPage),
            0xDA => self.movw_dp_ya(),
            0xDB => self.sty(AddressingMode::XIndexedDirectPage),
            0xDC => self.dec_reg(Register::Y),
            0xDD => self.tya(),
            0xDE => self.cbne(AddressingMode::XIndexedDirectPage),
            0xDF => self.daa(),

            0xE0 => self.clr_hv(),
            0xE1 => self.tcall_n(14),
            0xE2 => self.set_n_bit(7),
            0xE3 => self.bb_sc(7, true),
            0xE4 => self.lda(AddressingMode::DirectPage),
            0xE5 => self.lda(AddressingMode::Absolute),
            0xE6 => self.lda(AddressingMode::IndirectX),
            0xE7 => self.lda(AddressingMode::XIndexedIndirect),
            0xE8 => self.lda(AddressingMode::Immediate),
            0xE9 => self.ldx(AddressingMode::Absolute),
            0xEA => self.not1(),
            0xEB => self.ldy(AddressingMode::DirectPage),
            0xEC => self.ldy(AddressingMode::Absolute),
            0xED => self.notc(),
            0xEE => self.ply(),
            0xEF => self.sleep(),

            0xF0 => self.br(BranchType::Beq),
            0xF1 => self.tcall_n(15),
            0xF2 => self.clr_n_bit(7),
            0xF3 => self.bb_sc(7, false),
            0xF4 => self.lda(AddressingMode::XIndexedDirectPage),
            0xF5 => self.lda(AddressingMode::XIndexedAbsolute),
            0xF6 => self.lda(AddressingMode::YIndexedAbsolute),
            0xF7 => self.lda(AddressingMode::IndirectYIndexedIndirect),
            0xF8 => self.ldx(AddressingMode::DirectPage),
            0xF9 => self.ldx(AddressingMode::YIndexedDirectPage),
            0xFA => self.mov_dp_dp(),
            0xFB => self.ldy(AddressingMode::XIndexedDirectPage),
            0xFC => self.inc_reg(Register::Y),
            0xFD => self.tay(),
            0xFE => self.dbnz_y(),
            0xFF => self.stop(),
        }
        // debug!(
        //     "SPC: Councter: {}, PC:{:04X} op: {:02X} A:{:02X} X:{:02X} Y:{:02X} SP:{:02X} P:{}{}{}{}{}{}{}{} CYC:{}",
        //     self.instruction_counter,
        //     pc,
        //     op,
        //     self.registers.a,
        //     self.registers.x,
        //     self.registers.y,
        //     self.registers.sp,
        //     if self.registers.psw.n() { 'N' } else { 'n' },
        //     if self.registers.psw.v() { 'V' } else { 'v' },
        //     if self.registers.psw.p() { 'P' } else { 'p' },
        //     if self.registers.psw.b() { 'B' } else { 'b' },
        //     if self.registers.psw.h() { 'H' } else { 'h' },
        //     if self.registers.psw.i() { 'I' } else { 'i' },
        //     if self.registers.psw.z() { 'Z' } else { 'z' },
        //     if self.registers.psw.c() { 'C' } else { 'c' },
        //     self.counter,
        // );
        self.instruction_counter += 1;
    }

    fn read_8(&mut self, addr: WrapAddr) -> u8 {
        let addr = addr.addr;
        let data = match addr {
            0x0000..=0x00EF | 0x0100..=0xFFBF => {
                self.counter += self.io_registers.waitstate_on_ram_access;
                self.io_registers.dsp.ram[addr as usize]
            }
            0x00F0..=0x00FF => {
                self.counter += self.io_registers.waitstate_on_io_and_rom_access;
                self.io_registers.read((addr - 0xF0) as u8)
            }
            0xFFC0..=0xFFFF => {
                if self.io_registers.is_rom_read_enabled {
                    self.counter += self.io_registers.waitstate_on_io_and_rom_access;
                    ROM[(addr - 0xFFC0) as usize]
                } else {
                    self.counter += self.io_registers.waitstate_on_ram_access;
                    self.io_registers.dsp.ram[addr as usize]
                }
            }
        };
        // debug!("SPC Read:  {addr:#06X} = {data:#04X}");
        data
    }

    fn write_8(&mut self, addr: WrapAddr, data: u8) {
        let addr = addr.addr;
        // debug!("SPC Write: {addr:#06X} = {data:#04X}");
        // match addr {
        //     0x0000..=0x00EF | 0x0100..=0xFFFF => {
        //         self.counter += self.io_registers.waitstate_on_ram_access;
        //         warn!("Dsp ram write: {:#06X} = {:#X}", addr, data);
        //         self.io_registers.dsp.ram[addr as usize] = data;
        //     }
        //     0x00F0..=0x00FF => {
        //         self.counter += self.io_registers.waitstate_on_io_and_rom_access;
        //         self.io_registers.write((addr - 0xF0) as u8, data);
        //     }
        // }

        if self.io_registers.ram_write_enable {
            warn!("Dsp ram write: {:#06X} = {:#X}", addr, data);
            self.io_registers.dsp.ram[addr as usize] = data;
        }
        if addr & 0xFFF0 == 0x00F0 {
            self.io_registers.write((addr & 0xF) as u8, data);
            self.counter += self.io_registers.waitstate_on_io_and_rom_access;
        } else {
            self.counter += self.io_registers.waitstate_on_ram_access;
        }
    }

    fn write_16(&mut self, addr: WrapAddr, data: u16) {
        let lo = data as u8;
        let hi = (data >> 8) as u8;
        self.write_8(addr, lo);
        self.write_8(addr.offset(1), hi);
    }

    fn read_16(&mut self, addr: WrapAddr) -> u16 {
        let lo = self.read_8(addr);
        let hi = self.read_8(addr.offset(1));
        u16::from_le_bytes([lo, hi])
    }

    fn fetch_8(&mut self) -> u8 {
        let ret = self.read_8(WrapAddr {
            addr: self.registers.pc,
            wrap_mode: WrapMode::NoWrap,
        });
        self.registers.pc = self.registers.pc.wrapping_add(1);
        ret
    }

    fn fetch_16(&mut self) -> u16 {
        let lo = self.fetch_8();
        let hi = self.fetch_8();
        u16::from_le_bytes([lo, hi])
    }

    fn push_8(&mut self, data: u8) {
        self.write_8(
            WrapAddr {
                addr: 0x100 | u16::from(self.registers.sp),
                wrap_mode: WrapMode::Wrap8bit,
            },
            data,
        );
        self.registers.sp = self.registers.sp.wrapping_sub(1);
    }

    fn push_16(&mut self, data: u16) {
        self.push_8((data >> 8) as u8);
        self.push_8(data as u8);
    }

    fn pop_8(&mut self) -> u8 {
        self.registers.sp = self.registers.sp.wrapping_add(1);
        self.read_8(WrapAddr {
            addr: 0x100 | u16::from(self.registers.sp),
            wrap_mode: WrapMode::Wrap8bit,
        })
    }

    fn pop_16(&mut self) -> u16 {
        let lo = self.pop_8();
        let hi = self.pop_8();
        u16::from_le_bytes([lo, hi])
    }

    fn set_n(&mut self, val: u8) {
        self.registers.psw.set_n(val & 0x80 != 0);
    }

    fn set_z(&mut self, val: u8) {
        self.registers.psw.set_z(val == 0);
    }

    fn set_nz(&mut self, val: u8) {
        self.set_n(val);
        self.set_z(val);
    }

    fn set_n16(&mut self, val: u16) {
        self.registers.psw.set_n(val & 0x8000 != 0);
    }

    fn set_z16(&mut self, val: u16) {
        self.registers.psw.set_z(val == 0);
    }

    fn set_nz16(&mut self, val: u16) {
        self.set_n16(val);
        self.set_z16(val);
    }

    fn get_ya(&self) -> u16 {
        (self.registers.y as u16) << 8 | self.registers.a as u16
    }

    fn set_ya(&mut self, val: u16) {
        self.registers.a = val as u8;
        self.registers.y = (val >> 8) as u8;
    }
}

impl Spc {
    fn lda(&mut self, mode: AddressingMode) {
        let addr = self.get_warp_address(mode);
        self.registers.a = self.read_8(addr);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.set_nz(self.registers.a);
    }

    fn ldx(&mut self, mode: AddressingMode) {
        let addr = self.get_warp_address(mode);
        self.registers.x = self.read_8(addr);
        self.set_nz(self.registers.x);
    }

    fn ldy(&mut self, mode: AddressingMode) {
        let addr = self.get_warp_address(mode);
        self.registers.y = self.read_8(addr);
        self.set_nz(self.registers.y);
    }

    fn sta(&mut self, mode: AddressingMode) {
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        let addr = self.get_warp_address(mode);
        self.write_8(addr, self.registers.a);
    }

    fn stx(&mut self, mode: AddressingMode) {
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        let addr = self.get_warp_address(mode);
        self.write_8(addr, self.registers.x);
    }

    fn sty(&mut self, mode: AddressingMode) {
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        let addr = self.get_warp_address(mode);
        self.write_8(addr, self.registers.y);
    }

    fn txa(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.registers.a = self.registers.x;
        self.set_nz(self.registers.a);
    }

    fn tya(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.registers.a = self.registers.y;
        self.set_nz(self.registers.a);
    }

    fn tax(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.registers.x = self.registers.a;
        self.set_nz(self.registers.x);
    }

    fn tay(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.registers.y = self.registers.a;
        self.set_nz(self.registers.y);
    }

    fn tsx(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.registers.x = self.registers.sp;
        self.set_nz(self.registers.x);
    }

    fn txs(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.registers.sp = self.registers.x;
    }

    fn pha(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.push_8(self.registers.a);
    }

    fn phx(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.push_8(self.registers.x);
    }

    fn phy(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.push_8(self.registers.y);
    }

    fn php(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.push_8(self.registers.psw.into());
    }

    fn pla(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.registers.a = self.pop_8();
    }

    fn plx(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.registers.x = self.pop_8();
    }

    fn ply(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.registers.y = self.pop_8();
    }

    fn plp(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.registers.psw = self.pop_8().into();
    }

    fn daa(&mut self) {
        let src = self.registers.a;
        if self.registers.psw.h() || (src & 0x0F) > 9 {
            self.registers.a = self.registers.a.wrapping_add(6);
            if self.registers.a < 6 {
                self.registers.psw.set_c(true);
            }
        }
        if self.registers.psw.c() || (src > 0x99) {
            self.registers.a = self.registers.a.wrapping_add(0x60);
            self.registers.psw.set_c(true);
        }
        self.set_nz(self.registers.a);
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
    }

    fn das(&mut self) {
        let src = self.registers.a;
        if !self.registers.psw.h() || (src & 0x0F) > 9 {
            self.registers.a = self.registers.a.wrapping_sub(6);
        }
        if !self.registers.psw.c() || (src > 0x99) {
            self.registers.a = self.registers.a.wrapping_sub(0x60);
            self.registers.psw.set_c(false);
        }
        self.set_nz(self.registers.a);
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
    }

    fn bb_sc(&mut self, bit: u8, is_set: bool) {
        let addr = self.get_warp_address(AddressingMode::DirectPage);
        let v = self.read_8(addr);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        let offset = self.fetch_8() as i8 as u16;
        let dest = self.registers.pc.wrapping_add(offset);
        if (v & (1 << bit) != 0) == is_set {
            self.registers.pc = dest;
            self.increment_counter(self.io_registers.waitstate_on_ram_access * 2);
        }
    }

    fn alu(&mut self, alu_type: AluType, addressing_mode: AddressingMode) {
        match addressing_mode {
            AddressingMode::Immediate
            | AddressingMode::IndirectX
            | AddressingMode::DirectPage
            | AddressingMode::XIndexedDirectPage
            | AddressingMode::Absolute
            | AddressingMode::XIndexedAbsolute
            | AddressingMode::YIndexedAbsolute
            | AddressingMode::IndirectYIndexedIndirect
            | AddressingMode::XIndexedIndirect => {
                let addr = self.get_warp_address(addressing_mode);
                let operand = self.read_8(addr);
                let val = match alu_type {
                    AluType::Or => self.registers.a | operand,
                    AluType::And => self.registers.a & operand,
                    AluType::Eor => self.registers.a ^ operand,
                    AluType::Cmp => {
                        let (result, c) = self.registers.a.overflowing_sub(operand);
                        self.registers.psw.set_c(!c);
                        result
                    }
                    AluType::Adc => self.adc(self.registers.a, operand),
                    AluType::Sbc => self.sbc(self.registers.a, operand),
                };
                self.set_nz(val);
                if alu_type != AluType::Cmp {
                    self.registers.a = val;
                }
            }
            AddressingMode::DirectPageToDirectPage => {
                let operand2_addr = self.get_warp_address(AddressingMode::DirectPage);
                let operand2 = self.read_8(operand2_addr);
                let addr = self.get_warp_address(AddressingMode::DirectPage);
                let operand1 = self.read_8(addr);
                let val = match alu_type {
                    AluType::Or => operand1 | operand2,
                    AluType::And => operand1 & operand2,
                    AluType::Eor => operand1 ^ operand2,
                    AluType::Cmp => {
                        let (result, c) = operand1.overflowing_sub(operand2);
                        self.registers.psw.set_c(!c);
                        result
                    }
                    AluType::Adc => self.adc(operand1, operand2),
                    AluType::Sbc => self.sbc(operand1, operand2),
                };
                self.set_nz(val);
                if alu_type != AluType::Cmp {
                    self.write_8(addr, val);
                }
            }
            AddressingMode::ImmediateDataToDirectPage => {
                let operand2_addr = self.get_warp_address(AddressingMode::Immediate);
                let operand2 = self.read_8(operand2_addr);
                let addr = self.get_warp_address(AddressingMode::DirectPage);
                let operand1 = self.read_8(addr);
                let val = match alu_type {
                    AluType::Or => operand1 | operand2,
                    AluType::And => operand1 & operand2,
                    AluType::Eor => operand1 ^ operand2,
                    AluType::Cmp => {
                        let (result, c) = operand1.overflowing_sub(operand2);
                        self.registers.psw.set_c(!c);
                        result
                    }
                    AluType::Adc => self.adc(operand1, operand2),
                    AluType::Sbc => self.sbc(operand1, operand2),
                };
                self.set_nz(val);
                if alu_type != AluType::Cmp {
                    self.write_8(addr, val);
                }
            }
            AddressingMode::IndirectPageToIndirectPage => {
                let operand2_addr = self.get_warp_address(AddressingMode::IndirectY);
                let operand2 = self.read_8(operand2_addr);
                let addr = self.get_warp_address(AddressingMode::IndirectX);
                let operand1 = self.read_8(addr);
                let val = match alu_type {
                    AluType::Or => operand1 | operand2,
                    AluType::And => operand1 & operand2,
                    AluType::Eor => operand1 ^ operand2,
                    AluType::Cmp => {
                        let (result, c) = operand1.overflowing_sub(operand2);
                        self.registers.psw.set_c(!c);
                        result
                    }
                    AluType::Adc => self.adc(operand1, operand2),
                    AluType::Sbc => self.sbc(operand1, operand2),
                };
                self.set_nz(val);
                if alu_type != AluType::Cmp {
                    self.write_8(addr, val);
                }
            }
            _ => unreachable!("alu, mode: {:?}", addressing_mode),
        }
    }

    fn adc(&mut self, operand1: u8, operand2: u8) -> u8 {
        let v = operand1 as u32 + operand2 as u32 + self.registers.psw.c() as u32;
        let h = (operand1 & 0xF) + (operand2 & 0xF) + self.registers.psw.c() as u8;
        self.registers.psw.set_c(v > 0xFF);
        self.registers
            .psw
            .set_v((!(operand1 ^ operand2) & (operand1 ^ v as u8) & 0x80) != 0);
        self.registers.psw.set_h(h > 0xF);
        v as u8
    }

    fn sbc(&mut self, operand1: u8, operand2: u8) -> u8 {
        let v = (operand1 as u32)
            .wrapping_sub(operand2 as u32)
            .wrapping_sub(!self.registers.psw.c() as u32);
        let h = (operand1 & 0xF)
            .wrapping_sub(operand2 & 0xF)
            .wrapping_sub(!self.registers.psw.c() as u8);
        self.registers.psw.set_c(!(v > 0xFF));
        self.registers
            .psw
            .set_v(((operand1 ^ operand2) & (operand1 ^ v as u8) & 0x80) != 0);
        self.registers.psw.set_h(!(h > 0xF));
        v as u8
    }

    fn mov_dp_dp(&mut self) {
        let val_addr = self.get_warp_address(AddressingMode::DirectPage);
        let val = self.read_8(val_addr);
        let addr = self.get_warp_address(AddressingMode::DirectPage);
        self.write_8(addr, val);
    }

    fn mov_dp_imm(&mut self) {
        let val = self.fetch_8();
        let addr = self.get_warp_address(AddressingMode::DirectPage);
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.write_8(addr, val);
    }

    fn cmp_x(&mut self, mode: AddressingMode) {
        let addr = self.get_warp_address(mode);
        let operand = self.read_8(addr);
        let (result, c) = self.registers.x.overflowing_sub(operand);
        self.registers.psw.set_c(!c);
        self.set_nz(result);
    }

    fn cmp_y(&mut self, mode: AddressingMode) {
        let addr = self.get_warp_address(mode);
        let operand = self.read_8(addr);
        let (result, c) = self.registers.y.overflowing_sub(operand);
        self.registers.psw.set_c(!c);
        self.set_nz(result);
    }

    fn asl_a(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        let c = self.registers.a & 0x80 != 0;
        self.registers.a <<= 1;
        self.registers.psw.set_c(c);
        self.set_nz(self.registers.a);
    }

    fn asl_with_addressing(&mut self, mode: AddressingMode) {
        let addr = self.get_warp_address(mode);
        let val = self.read_8(addr);
        let c = val & 0x80 != 0;
        let val = val << 1;
        self.write_8(addr, val);
        self.registers.psw.set_c(c);
        self.set_nz(val);
    }

    fn rol_a(&mut self) {
        let val = self.registers.a << 1 | self.registers.psw.c() as u8;
        self.registers.psw.set_c(self.registers.a & 0x80 != 0);
        self.set_nz(val);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.registers.a = val;
    }

    fn rol_with_addressing(&mut self, mode: AddressingMode) {
        let addr = self.get_warp_address(mode);
        let operand = self.read_8(addr);
        let val = operand << 1 | self.registers.psw.c() as u8;
        self.registers.psw.set_c(operand & 0x80 != 0);
        self.set_nz(val);
        self.write_8(addr, val);
    }

    fn lsr_a(&mut self) {
        let c = self.registers.a & 0x01 != 0;
        self.registers.a >>= 1;
        self.registers.psw.set_c(c);
        self.set_nz(self.registers.a);
    }

    fn lsr_with_addressing(&mut self, mode: AddressingMode) {
        let addr = self.get_warp_address(mode);
        let val = self.read_8(addr);
        let c = val & 0x01 != 0;
        let val = val >> 1;
        self.write_8(addr, val);
        self.registers.psw.set_c(c);
        self.set_nz(val);
    }

    fn ror_a(&mut self) {
        let val = (self.registers.a >> 1) | ((self.registers.psw.c() as u8) << 7);
        self.registers.psw.set_c(self.registers.a & 0x01 != 0);
        self.set_nz(val);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.registers.a = val;
    }

    fn ror_with_addressing(&mut self, mode: AddressingMode) {
        let addr = self.get_warp_address(mode);
        let operand = self.read_8(addr);
        let val = (operand >> 1) | ((self.registers.psw.c() as u8) << 7);
        self.registers.psw.set_c(operand & 0x01 != 0);
        self.set_nz(val);
        self.write_8(addr, val);
    }

    fn dec_reg(&mut self, reg: Register) {
        let mut val = match reg {
            Register::A => self.registers.a,
            Register::X => self.registers.x,
            Register::Y => self.registers.y,
            _ => unreachable!("dec_reg, reg: {:?}", reg),
        };
        val = val.wrapping_sub(1);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.set_nz(val);
        match reg {
            Register::A => self.registers.a = val,
            Register::X => self.registers.x = val,
            Register::Y => self.registers.y = val,
            _ => unreachable!("dec_reg, reg: {:?}", reg),
        }
    }

    fn dec_with_addressing(&mut self, mode: AddressingMode) {
        let addr = self.get_warp_address(mode);
        let val = self.read_8(addr).wrapping_sub(1);
        self.set_nz(val);
        self.write_8(addr, val);
    }

    fn inc_reg(&mut self, reg: Register) {
        let mut val = match reg {
            Register::A => self.registers.a,
            Register::X => self.registers.x,
            Register::Y => self.registers.y,
            _ => unreachable!("inc_reg, reg: {:?}", reg),
        };
        val = val.wrapping_add(1);
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.set_nz(val);
        match reg {
            Register::A => self.registers.a = val,
            Register::X => self.registers.x = val,
            Register::Y => self.registers.y = val,
            _ => unreachable!("inc_reg, reg: {:?}", reg),
        }
    }

    fn inc_with_addressing(&mut self, mode: AddressingMode) {
        let addr = self.get_warp_address(mode);
        let val = self.read_8(addr).wrapping_add(1);
        self.set_nz(val);
        self.write_8(addr, val);
    }

    fn movw_ya_dp(&mut self) {
        let addr = self.get_warp_address(AddressingMode::DirectPage);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        let val = self.read_16(addr);
        self.set_nz16(val);
        self.set_ya(val);
    }

    fn movw_dp_ya(&mut self) {
        let addr = self.get_warp_address(AddressingMode::DirectPage);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        let ya = self.get_ya();
        self.write_16(addr, ya);
    }

    fn addw(&mut self) {
        let addr = self.get_warp_address(AddressingMode::DirectPage);
        let operand = self.read_16(addr) as u32;
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        let ya = self.get_ya() as u32;
        let v = ya.wrapping_add(operand);
        self.registers.psw.set_c(v > 0xFFFF);
        self.set_nz16(v as u16);
        self.registers
            .psw
            .set_v(!(ya ^ operand) & (ya ^ v) & 0x8000 != 0);
        let h = (ya & 0xFFF) + (operand & 0xFFF) > 0xFFF;
        self.registers.psw.set_h(h);
        self.set_ya(v as u16);
    }

    fn subw(&mut self) {
        let addr = self.get_warp_address(AddressingMode::DirectPage);
        let operand = self.read_16(addr) as u32;
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        let ya = self.get_ya() as u32;
        let v = ya.wrapping_sub(operand);
        self.registers.psw.set_c(!(v > 0xFFFF));
        self.set_nz16(v as u16);
        self.registers
            .psw
            .set_v((ya ^ operand) & (ya ^ v) & 0x8000 != 0);
        let h = (ya & 0xFFF).wrapping_sub(operand & 0xFFF) > 0xFFF;
        self.registers.psw.set_h(!h);
        self.set_ya(v as u16);
    }

    fn cmpw(&mut self) {
        let addr = self.get_warp_address(AddressingMode::DirectPage);
        // TODO no wrap?
        let operand = self.read_16(addr);
        let ya = self.get_ya();
        let (result, c) = ya.overflowing_sub(operand);
        self.registers.psw.set_c(!c);
        self.set_nz16(result);
    }

    fn incw(&mut self) {
        let addr = self.get_warp_address(AddressingMode::DirectPage);
        let lo = self.read_8(addr);
        self.write_8(addr, lo.wrapping_add(1));
        let addr = addr.offset(1);
        let hi = self.read_8(addr);
        let op = (hi as u16) << 8 | lo as u16;
        let val = op.wrapping_add(1);
        self.set_nz16(val);
        self.write_8(addr, (val >> 8) as u8);
    }

    fn decw(&mut self) {
        let addr = self.get_warp_address(AddressingMode::DirectPage);
        let lo = self.read_8(addr);
        self.write_8(addr, lo.wrapping_sub(1));
        let addr = addr.offset(1);
        let hi = self.read_8(addr);
        let op = (hi as u16) << 8 | lo as u16;
        let val = op.wrapping_sub(1);
        self.set_nz16(val);
        self.write_8(addr, (val >> 8) as u8);
    }

    fn div(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access * 10);
        let ya = self.get_ya();
        let x = u16::from(self.registers.x);
        if x > 0 {
            let quotient = ya / x;
            let remainder = ya % x;
            self.registers.a = quotient as u8;
            self.registers.y = remainder as u8;
            self.registers.psw.set_v(quotient > 0xFF);
            self.set_nz(quotient as u8);
        } else {
            self.registers.a = 0xFF;
            self.registers.y = 0xFF;
            self.registers.psw.set_z(false);
            self.registers.psw.set_v(true);
            self.registers.psw.set_n(true);
        }
    }

    fn mul(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access * 7);
        let val = (self.registers.a as u16) * (self.registers.y as u16);
        self.set_ya(val);
        self.set_nz(self.registers.y);
    }

    fn set_n_bit(&mut self, bit: u8) {
        let addr = self.get_warp_address(AddressingMode::DirectPage);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        let val = self.read_8(addr) | (1 << bit);
        self.write_8(addr, val);
    }

    fn clr_n_bit(&mut self, bit: u8) {
        let addr = self.get_warp_address(AddressingMode::DirectPage);
        let val = self.read_8(addr) & !(1 << bit);
        self.write_8(addr, val);
    }

    fn not1(&mut self) {
        let baaa = self.fetch_16();
        let b = baaa >> 13;
        let aaa = baaa & 0x1FFF;
        let addr = WrapAddr {
            addr: aaa,
            wrap_mode: WrapMode::NoWrap,
        };
        let operand = self.read_8(addr);
        let val = operand ^ (1 << b);
        self.write_8(addr, val);
    }

    fn mov1_aaa_b_c(&mut self) {
        let baaa = self.fetch_16();
        let b = baaa >> 13;
        let aaa = baaa & 0x1FFF;
        let addr = WrapAddr {
            addr: aaa,
            wrap_mode: WrapMode::NoWrap,
        };
        let operand = self.read_8(addr);
        let val = operand & !(1 << b) | (self.registers.psw.c() as u8) << b;
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.write_8(addr, val);
    }

    fn mov1_c_aaa_b(&mut self) {
        let baaa = self.fetch_16();
        let b = baaa >> 13;
        let aaa = baaa & 0x1FFF;
        let addr = WrapAddr {
            addr: aaa,
            wrap_mode: WrapMode::NoWrap,
        };
        let operand = self.read_8(addr);
        self.registers.psw.set_c(operand & (1 << b) != 0);
    }

    fn or1_c_aaa_b(&mut self) {
        let baaa = self.fetch_16();
        let b = baaa >> 13;
        let aaa = baaa & 0x1FFF;
        let addr = WrapAddr {
            addr: aaa,
            wrap_mode: WrapMode::NoWrap,
        };
        let operand = self.read_8(addr);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.registers
            .psw
            .set_c(self.registers.psw.c() || operand & (1 << b) != 0);
    }

    fn or_not1_c_aaa_b(&mut self) {
        let baaa = self.fetch_16();
        let b = baaa >> 13;
        let aaa = baaa & 0x1FFF;
        let addr = WrapAddr {
            addr: aaa,
            wrap_mode: WrapMode::NoWrap,
        };
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        let operand = self.read_8(addr);
        self.registers
            .psw
            .set_c(self.registers.psw.c() || operand & (1 << b) == 0);
    }

    fn and1_c_aaa_b(&mut self) {
        let baaa = self.fetch_16();
        let b = baaa >> 13;
        let aaa = baaa & 0x1FFF;
        let addr = WrapAddr {
            addr: aaa,
            wrap_mode: WrapMode::NoWrap,
        };
        let operand = self.read_8(addr);
        self.registers
            .psw
            .set_c(self.registers.psw.c() && operand & (1 << b) != 0);
    }

    fn and_not1_c_aaa_b(&mut self) {
        let baaa = self.fetch_16();
        let b = baaa >> 13;
        let aaa = baaa & 0x1FFF;
        let addr = WrapAddr {
            addr: aaa,
            wrap_mode: WrapMode::NoWrap,
        };
        let operand = self.read_8(addr);
        self.registers
            .psw
            .set_c(self.registers.psw.c() && operand & (1 << b) == 0);
    }

    fn eor1_c_aaa_b(&mut self) {
        let baaa = self.fetch_16();
        let b = baaa >> 13;
        let aaa = baaa & 0x1FFF;
        let addr = WrapAddr {
            addr: aaa,
            wrap_mode: WrapMode::NoWrap,
        };
        let operand = self.read_8(addr);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.registers
            .psw
            .set_c(self.registers.psw.c() ^ (operand & (1 << b) != 0));
    }

    fn clr_c(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.registers.psw.set_c(false);
    }

    fn set_c(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.registers.psw.set_c(true);
    }

    fn notc(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.registers.psw.set_c(!self.registers.psw.c());
    }

    fn clr_hv(&mut self) {
        self.registers.psw.set_h(false);
        self.registers.psw.set_v(false);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
    }

    fn xcn(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access * 3);
        self.registers.a = self.registers.a.rotate_right(4);
        self.set_nz(self.registers.a);
    }

    fn tclr(&mut self) {
        let addr = self.get_warp_address(AddressingMode::Absolute);
        let val = self.read_8(addr);
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.set_nz(self.registers.a.wrapping_sub(val));
        self.write_8(addr, val & !self.registers.a);
    }

    fn tset(&mut self) {
        let addr = self.get_warp_address(AddressingMode::Absolute);
        let val = self.read_8(addr);
        self.set_nz(self.registers.a.wrapping_sub(val));
        self.write_8(addr, val | self.registers.a);
    }

    fn br(&mut self, branch_type: BranchType) {
        let offset = self.fetch_8() as i8 as u16;
        if self.check_branch_condition(branch_type) {
            self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access * 2);
            self.registers.pc = self.registers.pc.wrapping_add(offset);
        }
    }

    fn check_branch_condition(&self, branch_type: BranchType) -> bool {
        match branch_type {
            BranchType::Bra => true,
            BranchType::Beq => self.registers.psw.z(),
            BranchType::Bne => !self.registers.psw.z(),
            BranchType::Bcs => self.registers.psw.c(),
            BranchType::Bcc => !self.registers.psw.c(),
            BranchType::Bvs => self.registers.psw.v(),
            BranchType::Bvc => !self.registers.psw.v(),
            BranchType::Bmi => self.registers.psw.n(),
            BranchType::Bpl => !self.registers.psw.n(),
        }
    }

    fn cbne(&mut self, addressing_mode: AddressingMode) {
        let addr = self.get_warp_address(addressing_mode);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        let operand = self.read_8(addr);
        let offset = self.fetch_8() as i8 as u16;
        if self.registers.a != operand {
            self.increment_counter(self.io_registers.waitstate_on_ram_access);
            self.registers.pc = self.registers.pc.wrapping_add(offset);
        }
    }

    fn dbnz_y(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.registers.y = self.registers.y.wrapping_sub(1);
        let offset = self.fetch_8() as i8 as u16;
        if self.registers.y != 0 {
            self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access * 2);
            self.registers.pc = self.registers.pc.wrapping_add(offset);
        }
    }

    fn dbnz_dp(&mut self) {
        let addr = self.get_warp_address(AddressingMode::DirectPage);
        let val = self.read_8(addr).wrapping_sub(1);
        self.write_8(addr, val);
        let offset = self.fetch_8() as i8 as u16;
        if val != 0 {
            self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access * 2);
            self.registers.pc = self.registers.pc.wrapping_add(offset);
        }
    }

    fn jmp_abs(&mut self) {
        let addr = self.fetch_16();
        self.registers.pc = addr;
    }

    fn jmp_x_abs(&mut self) {
        let addr = self.get_warp_address(AddressingMode::XIndexedAbsolute);
        let dest = self.read_16(addr);
        self.registers.pc = dest;
    }

    fn call(&mut self) {
        let addr = self.fetch_16();
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access * 3);
        self.push_16(self.registers.pc);
        self.registers.pc = addr;
    }

    fn tcall_n(&mut self, bit: u16) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access * 3);
        self.push_16(self.registers.pc);
        let addr = WrapAddr {
            addr: 0xFFDE - 2 * bit,
            wrap_mode: WrapMode::NoWrap,
        };
        self.registers.pc = self.read_16(addr);
    }

    fn pcall(&mut self) {
        let n = self.fetch_8() as u16;
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access * 2);
        self.push_16(self.registers.pc);
        self.registers.pc = 0xFF00 | n;
    }

    fn ret(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.registers.pc = self.pop_16();
    }

    fn reti(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.registers.psw = self.pop_8().into();
        self.registers.pc = self.pop_16();
    }

    fn brk(&mut self) {
        self.push_16(self.registers.pc);
        self.push_8(self.registers.psw.into());
        self.registers.psw.set_i(false);
        self.registers.psw.set_b(true);
        let addr = WrapAddr {
            addr: 0xFFDE,
            wrap_mode: WrapMode::NoWrap,
        };
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access * 2);
        self.registers.pc = self.read_16(addr);
    }

    fn nop(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
    }

    fn sleep(&mut self) {
        self.counter += self.io_registers.waitstate_on_ram_access;
        self.sleep = true;
        panic!("SPC sleep occurred");
    }

    fn stop(&mut self) {
        self.counter += self.io_registers.waitstate_on_ram_access;
        self.stop = true;
        panic!("SPC stop occurred");
    }

    fn clrp(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.registers.psw.set_p(false);
    }

    fn setp(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.registers.psw.set_p(true);
    }

    fn ei(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.registers.psw.set_i(true);
    }

    fn di(&mut self) {
        self.increment_counter(self.io_registers.waitstate_on_ram_access);
        self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
        self.registers.psw.set_i(false);
    }

    fn get_warp_address(&mut self, mode: AddressingMode) -> WrapAddr {
        match mode {
            AddressingMode::Immediate => {
                let addr = self.registers.pc;
                self.registers.pc = self.registers.pc.wrapping_add(1);
                WrapAddr {
                    addr,
                    wrap_mode: WrapMode::Wrap8bit,
                }
            }
            AddressingMode::DirectPage => {
                let addr = (self.registers.psw.p() as u16) << 8 | u16::from(self.fetch_8());
                WrapAddr {
                    addr,
                    wrap_mode: WrapMode::Wrap8bit,
                }
            }
            AddressingMode::XIndexedDirectPage => {
                let addr = (self.registers.psw.p() as u16) << 8
                    | u16::from(self.fetch_8().wrapping_add(self.registers.x));
                self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
                WrapAddr {
                    addr,
                    wrap_mode: WrapMode::Wrap8bit,
                }
            }
            AddressingMode::YIndexedDirectPage => {
                let addr = (self.registers.psw.p() as u16) << 8
                    | u16::from(self.fetch_8().wrapping_add(self.registers.y));
                WrapAddr {
                    addr,
                    wrap_mode: WrapMode::Wrap8bit,
                }
            }
            AddressingMode::IndirectX => {
                let addr = (self.registers.psw.p() as u16) << 8 | u16::from(self.registers.x);
                self.increment_counter(self.io_registers.waitstate_on_ram_access);
                WrapAddr {
                    addr,
                    wrap_mode: WrapMode::Wrap8bit,
                }
            }
            AddressingMode::IndirectY => {
                let addr = (self.registers.psw.p() as u16) << 8 | u16::from(self.registers.y);
                self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
                WrapAddr {
                    addr,
                    wrap_mode: WrapMode::Wrap8bit,
                }
            }
            AddressingMode::IndirectAutoIncrement => {
                let addr = (self.registers.psw.p() as u16) << 8 | u16::from(self.registers.x);
                self.increment_counter(self.io_registers.waitstate_on_ram_access);
                self.registers.x = self.registers.x.wrapping_add(1);
                WrapAddr {
                    addr,
                    wrap_mode: WrapMode::Wrap8bit,
                }
            }
            AddressingMode::Absolute => {
                // let addr = self.registers.pc;
                // self.registers.pc = self.registers.pc.wrapping_add(2);
                let addr = self.fetch_16();
                WrapAddr {
                    addr,
                    wrap_mode: WrapMode::NoWrap,
                }
            }
            AddressingMode::XIndexedAbsolute => {
                let addr = self.fetch_16().wrapping_add(u16::from(self.registers.x));
                self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
                WrapAddr {
                    addr,
                    wrap_mode: WrapMode::NoWrap,
                }
            }
            AddressingMode::YIndexedAbsolute => {
                let addr = self.fetch_16().wrapping_add(u16::from(self.registers.y));
                self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
                WrapAddr {
                    addr,
                    wrap_mode: WrapMode::NoWrap,
                }
            }
            AddressingMode::XIndexedIndirect => {
                let wrap_addr = WrapAddr {
                    addr: (self.registers.psw.p() as u16) << 8
                        | self.fetch_8().wrapping_add(self.registers.x) as u16,
                    wrap_mode: WrapMode::NoWrap,
                };
                let addr = self.read_16(wrap_addr);
                self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
                WrapAddr {
                    addr,
                    wrap_mode: WrapMode::NoWrap,
                }
            }
            AddressingMode::IndirectYIndexedIndirect => {
                let wrap_addr = WrapAddr {
                    addr: (self.registers.psw.p() as u16) << 8 | self.fetch_8() as u16,
                    wrap_mode: WrapMode::NoWrap,
                };
                self.increment_counter(self.io_registers.waitstate_on_io_and_rom_access);
                let addr = self
                    .read_16(wrap_addr)
                    .wrapping_add(u16::from(self.registers.y));
                WrapAddr {
                    addr,
                    wrap_mode: WrapMode::NoWrap,
                }
            }
            _ => todo!("get_warp_address, mode: {:?}", mode),
        }
    }
}

#[derive(Debug)]
enum Register {
    A,
    X,
    Y,
    Psw,
    Sp,
    Pc,
}

#[derive(Debug)]
enum BranchType {
    Bra,
    Beq,
    Bne,
    Bcs,
    Bcc,
    Bvs,
    Bvc,
    Bmi,
    Bpl,
}

#[derive(Debug, PartialEq)]
enum AluType {
    Or,
    And,
    Eor,
    Cmp,
    Adc,
    Sbc,
}

#[derive(Debug)]
enum AddressingMode {
    Immediate,
    DirectPage,
    XIndexedDirectPage,
    YIndexedDirectPage,
    IndirectX,
    IndirectY,
    IndirectAutoIncrement,
    DirectPageToDirectPage,
    IndirectPageToIndirectPage,
    ImmediateDataToDirectPage,
    Absolute,
    XIndexedAbsolute,
    YIndexedAbsolute,
    XIndexedIndirect,
    IndirectYIndexedIndirect,
}

#[derive(Clone, Copy)]
struct WrapAddr {
    addr: u16,
    wrap_mode: WrapMode,
}

impl WrapAddr {
    fn offset(&self, offset: u16) -> Self {
        match self.wrap_mode {
            WrapMode::NoWrap => WrapAddr {
                addr: self.addr.wrapping_add(offset),
                wrap_mode: WrapMode::NoWrap,
            },
            WrapMode::Wrap8bit => WrapAddr {
                addr: (self.addr & 0xFF00) | ((self.addr + offset) & 0xFF),
                wrap_mode: WrapMode::Wrap8bit,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum WrapMode {
    NoWrap,
    Wrap8bit,
}

struct Registers {
    a: u8,
    x: u8,
    y: u8,
    psw: Psw,
    sp: u8,
    pc: u16,
}

impl Default for Registers {
    fn default() -> Self {
        Registers {
            a: 0,
            x: 0,
            y: 0,
            psw: Psw::default(),
            sp: 0xFF,
            pc: u16::from_le_bytes(ROM[0x3E..0x40].try_into().unwrap()),
        }
    }
}

#[bitfield(bits = 8)]
#[repr(u8)]
#[derive(Default, Debug, Clone, Copy)]
struct Psw {
    c: bool,
    z: bool,
    i: bool,
    h: bool,
    b: bool,
    p: bool,
    v: bool,
    n: bool,
}

struct IORegisters {
    waitstate_on_ram_access: u64,
    waitstate_on_io_and_rom_access: u64,
    cpu_in: [u8; 4],
    cpu_out: [u8; 4],
    is_rom_read_enabled: bool,
    ram_write_enable: bool,

    dsp_addr: u8, // 0x00..=0x7F (0x80..=0xFF is mirror)
    pub dsp: dsp::Dsp,
    external_io_port: [u8; 2],
    timer: [Timer; 3],
    timer_counter_01: u64,
    timer_counter_2: u64,
}

impl Default for IORegisters {
    fn default() -> Self {
        IORegisters {
            waitstate_on_ram_access: 1,
            waitstate_on_io_and_rom_access: 1,
            cpu_in: [0; 4],
            cpu_out: [0; 4],
            is_rom_read_enabled: true,
            ram_write_enable: true,

            dsp_addr: 0,
            dsp: dsp::Dsp::default(),
            external_io_port: [0; 2],
            timer: [Timer::default(); 3],
            timer_counter_01: 0,
            timer_counter_2: 0,
        }
    }
}

impl IORegisters {
    fn read(&mut self, index: u8) -> u8 {
        match index {
            2 => self.dsp_addr,
            // 3 => self.dsp.ram[self.dsp_addr as usize],
            3 => self.dsp.read(self.dsp_addr),
            4..=7 => {
                let port = index - 4;
                let data = self.cpu_in[port as usize];
                // debug!("CPUIO {port} -> {data:#04X} ");
                data
            }
            8 | 9 => self.external_io_port[(index - 8) as usize],
            0xD..=0xF => self.timer[(index - 0xD) as usize].output(),
            _ => unreachable!("IORegisters Invalid read index: {:#X}", index),
        }
    }

    fn write(&mut self, index: u8, data: u8) {
        match index {
            0 => {
                // TODO Check the impakt of this bit
                // 0    Timer-Enable     (0=Normal, 1=Timers don't work)
                // 1    RAM Write Enable (0=Disable/Read-only, 1=Enable SPC700 & S-DSP writes)
                // 2    Crash SPC700     (0=Normal, 1=Crashes the CPU)
                // 3    Timer-Disable    (0=Timers don't work, 1=Normal)
                const CYCLE: [u64; 4] = [1, 2, 5, 10];
                self.ram_write_enable = data & 2 != 0;
                self.waitstate_on_ram_access = CYCLE[((data >> 4) & 0b11) as usize];
                self.waitstate_on_io_and_rom_access = CYCLE[((data >> 6) & 0b11) as usize];
            }
            1 => {
                for i in 0..3 {
                    self.timer[i].set_enabled(data & (1 << i) != 0);
                }

                for i in 0..2 {
                    if data & (1 << (i + 4)) != 0 {
                        self.cpu_in[i] = 0;
                        self.cpu_in[i + 1] = 0;
                    }
                }

                self.is_rom_read_enabled = data & 0x80 != 0;
            }
            //  2 => sef.dsp_addr = data & 0x7F,
            2 => self.dsp_addr = data,
            // 3 => self.dsp.ram[self.dsp_addr as usize] = data,
            3 => self.dsp.write(self.dsp_addr, data),
            4..=7 => {
                let port = index - 4;
                // debug!("CPUIO {port} <- {data:#04X}");
                self.cpu_out[(index - 4) as usize] = data;
            }
            8 | 9 => self.external_io_port[(index - 8) as usize] = data,
            0xA..=0xC => self.timer[(index - 0xA) as usize].set_divider(data),
            // _ => unreachable!(
            //     "IORegisters Invalid write index: {:#X}, data: {:X}",
            //     index, data
            // ),
            0xD..=0xF => {
                // debug!("Timer {} <- {data:#04X}", index - 0xD);
            }
            _ => unreachable!(
                "IORegisters Invalid write index: {:#X}, data: {:X}",
                index, data
            ),
        }
    }

    fn tick_timer(&mut self, elapsed: u64) {
        self.timer_counter_01 += elapsed;
        self.timer_counter_2 += elapsed;

        while self.timer_counter_01 >= 128 {
            self.timer_counter_01 -= 128;
            for i in 0..2 {
                self.timer[i].tick();
            }
        }
        while self.timer_counter_2 >= 16 {
            self.timer_counter_2 -= 16;
            self.timer[2].tick();
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Timer {
    is_enabled: bool,
    counter: u8,
    divider: u8,
    output: u8,
}

impl Default for Timer {
    fn default() -> Self {
        Timer {
            is_enabled: false,
            counter: 0xFF,
            divider: 0,
            output: 0,
        }
    }
}

impl Timer {
    fn set_enabled(&mut self, enabled: bool) {
        self.is_enabled = enabled;
    }

    fn set_divider(&mut self, divider: u8) {
        self.divider = divider;
    }

    fn output(&mut self) -> u8 {
        let ret = self.output;
        self.output = 0;
        ret
    }

    fn tick(&mut self) {
        if self.is_enabled {
            self.counter = self.counter.wrapping_add(1);
            if self.counter == self.divider {
                self.counter = 0;
                self.output = (self.output + 1) & 0xF;
            }
        }
    }
}
