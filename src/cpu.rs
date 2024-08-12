use crate::context;
trait Context: context::Bus + context::Ppu + context::Timing {}
impl<T: context::Bus + context::Ppu + context::Timing> Context for T {}

const CPU_CYCLE: u64 = 6;

pub struct Cpu {
    a: u16,
    x: u16,
    y: u16,
    pc: u16,
    s: u16,
    p: Status,
    d: u16,
    db: u8,
    pb: u8,
    e: bool,
}

impl Default for Cpu {
    fn default() -> Self {
        Cpu {
            a: 0,
            x: 0,
            y: 0,
            pc: 0,
            s: 0x01FF,
            p: Status::default(),
            d: 0,
            db: 0,
            pb: 0,
            e: true,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Status {
    c: bool,
    z: bool,
    i: bool,
    d: bool,
    x: bool,
    m: bool,
    v: bool,
    n: bool,
}

impl From<u8> for Status {
    fn from(data: u8) -> Self {
        Status {
            c: (data >> 7) & 1 == 1,
            z: (data >> 6) & 1 == 1,
            i: (data >> 5) & 1 == 1,
            d: (data >> 4) & 1 == 1,
            x: (data >> 3) & 1 == 1,
            m: (data >> 2) & 1 == 1,
            v: (data >> 1) & 1 == 1,
            n: (data >> 0) & 1 == 1,
        }
    }
}

impl Into<u8> for Status {
    fn into(self) -> u8 {
        let mut data = 0;
        data |= (self.c as u8) << 7;
        data |= (self.z as u8) << 6;
        data |= (self.i as u8) << 5;
        data |= (self.d as u8) << 4;
        data |= (self.x as u8) << 3;
        data |= (self.m as u8) << 2;
        data |= (self.v as u8) << 1;
        data |= (self.n as u8) << 0;
        data
    }
}

impl Default for Status {
    fn default() -> Self {
        Status {
            c: false,
            z: false,
            i: true,
            d: false,
            x: true,
            m: true,
            v: false,
            n: false,
        }
    }
}

enum Register {
    A,
    X,
    Y,
    PC,
    S,
    P,
    D,
    DB,
    PD,
}

trait Value: Copy {
    fn zero(&self) -> bool;
    fn negative(&self) -> bool;
}

impl Value for u8 {
    fn zero(&self) -> bool {
        *self == 0
    }
    fn negative(&self) -> bool {
        (*self >> 7) & 1 == 1
    }
}

impl Value for u16 {
    fn zero(&self) -> bool {
        *self == 0
    }
    fn negative(&self) -> bool {
        (*self >> 15) & 1 == 1
    }
}

#[derive(Debug, Clone, Copy)]
enum WarpMode {
    Warp8bit,
    Warp16bit,
    NoWarp,
}
struct WarpAddress {
    addr: u32,
    mode: WarpMode,
}

impl WarpAddress {
    fn unwrap(&self) -> u32 {
        self.addr
    }

    fn offset(&self, offset: u16) -> Self {
        let addr = match self.mode {
            WarpMode::Warp8bit => {
                self.addr & 0xFFFF00 | (self.addr as u8).wrapping_add(offset as u8) as u32
            }
            WarpMode::Warp16bit => {
                self.addr & 0xFF0000 | (self.addr as u16).wrapping_add(offset) as u32
            }
            WarpMode::NoWarp => (self.addr + offset as u32) & 0xFFFFF,
        };
        WarpAddress {
            addr,
            mode: self.mode,
        }
    }

    fn read_8(&self, context: &impl Context) -> u8 {
        context.bus_read(self.unwrap())
    }

    fn read_16(&self, context: &impl Context) -> u16 {
        let lo = context.bus_read(self.unwrap()) as u16;
        let hi = context.bus_read(self.offset(1).unwrap()) as u16;
        hi << 8 | lo
    }

    fn read_24(&self, context: &impl Context) -> u32 {
        let lo = context.bus_read(self.unwrap()) as u32;
        let hi = context.bus_read(self.offset(1).unwrap()) as u32;
        let bank = context.bus_read(self.offset(2).unwrap()) as u32;
        bank << 16 | hi << 8 | lo
    }

    fn write_8(&self, context: &mut impl Context, data: u8) {
        context.bus_write(self.unwrap(), data);
    }

    fn write_16(&self, context: &mut impl Context, data: u16) {
        context.bus_write(self.unwrap(), data as u8);
        context.bus_write(self.offset(1).unwrap(), (data >> 8) as u8);
    }
}

enum AddressingMode {
    Immediate,
    Absolute,
    AbsoluteLong,
    Direct,
    Accumulator,
    Implied,
    DirectIndirectIndexedY,
    DirectIndirectIndexedLongY,
    DirectIndexedIndirect,
    DirectX,
    DirectY,
    AbsoluteX,
    AbsoluteLongX,
    AbsoluteY,
    Relative,
    RelativeLong,
    AbsoluteIndirect,
    DirectIndirect,
    DirectIndirectLong,
    AbsoluteIndexedIndirect,
    Stack,
    StackRelative,
    StackRelativeIndirectIndexed,
    BlockMove,
}

enum AluType {
    Or,
    And,
    Xor,
    Add,
    Sub,
    Cmp,
}

enum BranchType {
    Bpl,
    Bmi,
    Bvc,
    Bvs,
    Bcc,
    Bcs,
    Bne,
    Beq,
}

impl Cpu {
    fn get_pc24(&self) -> u32 {
        (self.pb as u32) << 16 | self.pc as u32
    }
    fn fetch_8(&mut self, ctx: &mut impl Context) -> u8 {
        let data = ctx.bus_read(self.get_pc24());
        self.pc = self.pc.wrapping_add(1);
        data
    }

    fn fetch_16(&mut self, ctx: &mut impl Context) -> u16 {
        let lo = self.fetch_8(ctx) as u16;
        let hi = self.fetch_8(ctx) as u16;
        hi << 8 | lo
    }

    fn fetch_24(&mut self, ctx: &mut impl Context) -> u32 {
        let lo = self.fetch_8(ctx) as u32;
        let hi = self.fetch_8(ctx) as u32;
        let bank = self.fetch_8(ctx) as u32;
        bank << 16 | hi << 8 | lo
    }

    fn push_8(&mut self, ctx: &mut impl Context, data: u8) {
        ctx.bus_write(self.s as u32, data);
        if self.e {
            self.s = self.s & 0xFF00 | (self.s as u8).wrapping_sub(1) as u16;
        } else {
            self.s = self.s.wrapping_sub(1);
        }
    }

    fn push_16(&mut self, ctx: &mut impl Context, data: u16) {
        self.push_8(ctx, (data >> 8) as u8);
        self.push_8(ctx, data as u8);
    }

    fn pop_8(&mut self, ctx: &impl Context) -> u8 {
        if self.e {
            self.s = self.s & 0xFF00 | (self.s as u8).wrapping_add(1) as u16;
        } else {
            self.s = self.s.wrapping_add(1);
        }
        ctx.bus_read(self.s as u32)
    }

    fn pop_16(&mut self, ctx: &impl Context) -> u16 {
        let lo = self.pop_8(ctx) as u16;
        let hi = self.pop_8(ctx) as u16;
        hi << 8 | lo
    }

    fn set_n(&mut self, data: impl Value) {
        self.p.n = data.negative();
    }

    fn set_z(&mut self, data: impl Value) {
        self.p.z = data.zero();
    }

    fn set_nz(&mut self, data: impl Value) {
        self.set_n(data);
        self.set_z(data);
    }

    fn is_wrap8(&self) -> bool {
        self.e && (self.d & 0xFF) == 0
    }

    fn is_a_register_8bit(&self) -> bool {
        self.e || self.p.m
    }

    fn is_memory_8bit(&self) -> bool {
        self.e || self.p.m
    }

    fn is_xy_register_8bit(&self) -> bool {
        self.e || self.p.x
    }

    fn get_warp_address(
        &mut self,
        addressing_mode: AddressingMode,
        ctx: &mut impl Context,
    ) -> WarpAddress {
        match addressing_mode {
            //  AddressingMode::Immediate は別で扱う？
            AddressingMode::Absolute => {
                let addr = (self.pb as u32) << 16 | self.fetch_16(ctx) as u32;
                WarpAddress {
                    addr,
                    mode: WarpMode::NoWarp,
                }
            }
            AddressingMode::AbsoluteLong => {
                let addr = self.fetch_24(ctx);
                WarpAddress {
                    addr,
                    mode: WarpMode::NoWarp,
                }
            }
            AddressingMode::Direct => {
                let offset = self.fetch_8(ctx) as u16;
                if self.is_wrap8() {
                    WarpAddress {
                        addr: (self.d & 0xFF00 | offset) as u32,
                        mode: WarpMode::Warp8bit,
                    }
                } else {
                    if self.d & 0xFF != 0 {
                        ctx.elapse(CPU_CYCLE);
                    }
                    WarpAddress {
                        addr: self.d as u32,
                        mode: WarpMode::Warp16bit,
                    }
                    .offset(offset)
                }
            }
            // Accumulator
            // Implied
            AddressingMode::DirectIndirectIndexedY => {
                let offset = self.fetch_8(ctx);
                let direct_addr = if self.is_wrap8() {
                    WarpAddress {
                        addr: (self.d & 0xFF00 | offset as u16) as u32,
                        mode: WarpMode::Warp8bit,
                    }
                    .read_16(ctx)
                } else {
                    if self.d & 0xFF != 0 {
                        ctx.elapse(CPU_CYCLE);
                    }
                    WarpAddress {
                        addr: self.d as u32,
                        mode: WarpMode::Warp16bit,
                    }
                    .offset(offset as u16)
                    .read_16(ctx)
                };
                WarpAddress {
                    addr: (self.db as u32) << 16 | direct_addr as u32,
                    mode: WarpMode::NoWarp,
                }
                .offset(self.y)
            }
            AddressingMode::DirectIndirectIndexedLongY => {
                let offset = self.fetch_8(ctx);
                let direct_addr = if self.is_wrap8() {
                    WarpAddress {
                        addr: (self.d & 0xFF00 | offset as u16) as u32,
                        mode: WarpMode::Warp8bit,
                    }
                    .read_24(ctx)
                } else {
                    if self.d & 0xFF != 0 {
                        ctx.elapse(CPU_CYCLE);
                    }
                    WarpAddress {
                        addr: self.d as u32,
                        mode: WarpMode::Warp16bit,
                    }
                    .offset(offset as u16)
                    .read_24(ctx)
                };
                WarpAddress {
                    addr: direct_addr,
                    mode: WarpMode::NoWarp,
                }
                .offset(self.y)
            }
            AddressingMode::DirectIndexedIndirect => {
                let offset = self.fetch_8(ctx);
                if self.d & 0xFF != 0 {
                    ctx.elapse(CPU_CYCLE);
                }
                let mid_addr = if self.is_wrap8() {
                    WarpAddress {
                        addr: self.d as u32,
                        mode: WarpMode::Warp8bit,
                    }
                    .offset(offset as u16)
                    .offset(self.x)
                    .read_16(ctx)
                } else {
                    WarpAddress {
                        addr: self.d as u32,
                        mode: WarpMode::Warp16bit,
                    }
                    .offset(offset as u16)
                    .offset(self.x)
                    .read_16(ctx)
                };
                WarpAddress {
                    addr: (self.db as u32) << 16 | mid_addr as u32,
                    mode: WarpMode::NoWarp,
                }
            }
            AddressingMode::DirectX => {
                let offset = self.fetch_8(ctx) as u16;
                if self.d & 0xFF != 0 {
                    ctx.elapse(CPU_CYCLE);
                }
                if self.is_wrap8() {
                    WarpAddress {
                        addr: (self.d & 0xFF00 | offset) as u32,
                        mode: WarpMode::Warp8bit,
                    }
                    .offset(self.x)
                } else {
                    WarpAddress {
                        addr: self.d as u32,
                        mode: WarpMode::Warp16bit,
                    }
                    .offset(offset)
                    .offset(self.x)
                }
            }
            AddressingMode::DirectY => {
                let offset = self.fetch_8(ctx) as u16;
                if self.d & 0xFF != 0 {
                    ctx.elapse(CPU_CYCLE);
                }
                if self.is_wrap8() {
                    WarpAddress {
                        addr: (self.d & 0xFF00 | offset) as u32,
                        mode: WarpMode::Warp8bit,
                    }
                    .offset(self.y)
                } else {
                    WarpAddress {
                        addr: self.d as u32,
                        mode: WarpMode::Warp16bit,
                    }
                    .offset(offset)
                    .offset(self.y)
                }
            }
            AddressingMode::AbsoluteX => {
                let addr = self.fetch_16(ctx);
                WarpAddress {
                    addr: (self.db as u32) << 16 | addr as u32,
                    mode: WarpMode::NoWarp,
                }
                .offset(self.x)
            }
            AddressingMode::AbsoluteY => {
                let addr = self.fetch_16(ctx);
                WarpAddress {
                    addr: (self.db as u32) << 16 | addr as u32,
                    mode: WarpMode::NoWarp,
                }
                .offset(self.y)
            }
            // Relative
            // RelativeLong
            // AbsoluteIndirect
            AddressingMode::DirectIndirect => {
                let offset = self.fetch_8(ctx) as u16;
                if self.d & 0xFF != 0 {
                    ctx.elapse(CPU_CYCLE);
                }
                let addr = WarpAddress {
                    addr: self.d as u32,
                    mode: WarpMode::Warp16bit,
                }
                .offset(offset)
                .read_16(ctx);

                WarpAddress {
                    addr: (self.db as u32) << 16 | addr as u32,
                    mode: WarpMode::NoWarp,
                }
            }
            AddressingMode::DirectIndirectLong => {
                let offset = self.fetch_8(ctx) as u16;
                if self.d & 0xFF != 0 {
                    ctx.elapse(CPU_CYCLE);
                }
                let addr = WarpAddress {
                    addr: self.d as u32,
                    mode: WarpMode::Warp16bit,
                }
                .offset(offset)
                .read_24(ctx);

                WarpAddress {
                    addr,
                    mode: WarpMode::NoWarp,
                }
            }
            AddressingMode::AbsoluteIndexedIndirect => {
                let offset = self.fetch_16(ctx).wrapping_add(self.x);
                WarpAddress {
                    addr: (self.pb as u32) << 16 | offset as u32,
                    mode: WarpMode::Warp16bit,
                }
            }
            // Stack
            AddressingMode::StackRelative => {
                let offset = self.fetch_8(ctx) as u16;
                WarpAddress {
                    addr: self.s as u32,
                    mode: WarpMode::Warp16bit,
                }
                .offset(offset)
            }
            AddressingMode::StackRelativeIndirectIndexed => {
                let offset = self.fetch_8(ctx) as u16;
                ctx.elapse(CPU_CYCLE);
                let addr = WarpAddress {
                    addr: self.s as u32,
                    mode: WarpMode::Warp16bit,
                }
                .offset(offset)
                .read_16(ctx);
                WarpAddress {
                    addr: (self.db as u32) << 16 | addr as u32,
                    mode: WarpMode::NoWarp,
                }
                .offset(self.y)
            }
            // BlockMove
            _ => unimplemented!(),
        }
    }

    fn excecute_instruction(&mut self, ctx: &mut impl Context) {
        let opcode = self.fetch_8(ctx);
        match opcode {
            0x06 => self.asl_with_addressing(ctx, AddressingMode::Direct),
            0x08 => self.php(ctx),
            0x0A => self.asl_a(ctx),
            0x0B => self.phd(ctx),
            0x0E => self.asl_with_addressing(ctx, AddressingMode::Absolute),

            0x10 => self.cond_branch(ctx, BranchType::Bpl),
            0x16 => self.asl_with_addressing(ctx, AddressingMode::DirectX),
            0x1B => self.tcs(ctx),
            0x1E => self.asl_with_addressing(ctx, AddressingMode::AbsoluteX),

            0x20 => self.jsr_abs(ctx),
            0x22 => self.jsl_far(ctx),
            0x26 => self.rol_with_addressing(ctx, AddressingMode::Direct),
            0x28 => self.plp(ctx),
            0x2A => self.rol_a(ctx),
            0x2B => self.pld(ctx),
            0x2E => self.rol_with_addressing(ctx, AddressingMode::Absolute),

            0x30 => self.cond_branch(ctx, BranchType::Bmi),
            0x36 => self.rol_with_addressing(ctx, AddressingMode::DirectX),
            0x3B => self.tsc(ctx),
            0x3E => self.rol_with_addressing(ctx, AddressingMode::AbsoluteX),

            0x40 => self.rti(ctx),
            0x46 => self.lsr_with_addressing(ctx, AddressingMode::Direct),
            0x48 => self.pha(ctx),
            0x4A => self.lsr_a(ctx),
            0x4B => self.phk(ctx),
            0x4C => self.jmp_abs(ctx),
            0x4E => self.lsr_with_addressing(ctx, AddressingMode::Absolute),

            0x50 => self.cond_branch(ctx, BranchType::Bvc),
            0x56 => self.lsr_with_addressing(ctx, AddressingMode::DirectX),
            0x5A => self.phy(ctx),
            0x5B => self.tcd(ctx),
            0x5C => self.jmp_abs_long(ctx),
            0x5E => self.lsr_with_addressing(ctx, AddressingMode::AbsoluteX),

            0x60 => self.rts(ctx),
            0x62 => self.per(ctx),
            0x64 => self.stz(ctx, AddressingMode::Direct),
            0x66 => self.ror_with_addressing(ctx, AddressingMode::Direct),
            0x68 => self.pla(ctx),
            0x6A => self.ror_a(ctx),
            0x6B => self.rtl(ctx),
            0x6C => self.jmp_nnnn(ctx),
            0x6E => self.ror_with_addressing(ctx, AddressingMode::Absolute),

            0x70 => self.cond_branch(ctx, BranchType::Bvs),
            0x74 => self.stz(ctx, AddressingMode::DirectX),
            0x76 => self.ror_with_addressing(ctx, AddressingMode::DirectX),
            0x7A => self.ply(ctx),
            0x7B => self.tdc(ctx),
            0x7C => self.jmp_nnnn_x(ctx),
            0x7E => self.ror_with_addressing(ctx, AddressingMode::AbsoluteX),

            0x80 => self.jmp_disp_8(ctx),
            0x81 => self.sta(ctx, AddressingMode::DirectIndexedIndirect),
            0x82 => self.jmp_disp_16(ctx),
            0x83 => self.lda(ctx, AddressingMode::StackRelative),
            0x84 => self.sty(ctx, AddressingMode::Direct),
            0x85 => self.sta(ctx, AddressingMode::Direct),
            0x86 => self.stx(ctx, AddressingMode::Direct),
            0x87 => self.sta(ctx, AddressingMode::DirectIndirectLong),
            0x8A => self.txa(ctx),
            0x8B => self.phb(ctx),
            0x8C => self.sty(ctx, AddressingMode::Absolute),
            0x8D => self.sta(ctx, AddressingMode::Absolute),
            0x8E => self.stx(ctx, AddressingMode::Absolute),
            0x8F => self.sta(ctx, AddressingMode::AbsoluteLong),

            0x90 => self.cond_branch(ctx, BranchType::Bcc),
            0x91 => self.sta(ctx, AddressingMode::DirectIndirectIndexedY),
            0x92 => self.sta(ctx, AddressingMode::DirectIndirect),
            0x93 => self.lda(ctx, AddressingMode::StackRelativeIndirectIndexed),
            0x94 => self.sty(ctx, AddressingMode::DirectX),
            0x95 => self.sta(ctx, AddressingMode::DirectX),
            0x96 => self.stx(ctx, AddressingMode::DirectY),
            0x97 => self.sta(ctx, AddressingMode::DirectIndirectIndexedLongY),
            0x98 => self.tya(ctx),
            0x99 => self.sta(ctx, AddressingMode::AbsoluteY),
            0x9A => self.txs(ctx),
            0x9B => self.txy(ctx),
            0x9C => self.stz(ctx, AddressingMode::Absolute),
            0x9D => self.sta(ctx, AddressingMode::AbsoluteX),
            0x9E => self.stz(ctx, AddressingMode::AbsoluteX),
            0x9F => self.sta(ctx, AddressingMode::AbsoluteLongX),

            0xA0 => self.ldy_imm(ctx),
            0xA1 => self.lda(ctx, AddressingMode::DirectIndexedIndirect),
            0xA2 => self.ldx_imm(ctx),
            0xA3 => self.lda(ctx, AddressingMode::StackRelative),
            0xA4 => self.ldy(ctx, AddressingMode::Direct),
            0xA5 => self.lda(ctx, AddressingMode::Direct),
            0xA6 => self.ldx(ctx, AddressingMode::Direct),
            0xA7 => self.lda(ctx, AddressingMode::DirectIndirectLong),

            0xA8 => self.tay(ctx),
            0xA9 => self.lda_imm(ctx),
            0xAA => self.tax(ctx),
            0xAB => self.plb(ctx),
            0xAC => self.ldy(ctx, AddressingMode::Absolute),
            0xAD => self.lda(ctx, AddressingMode::Absolute),
            0xAE => self.ldx(ctx, AddressingMode::Absolute),
            0xAF => self.lda(ctx, AddressingMode::AbsoluteLong),

            0xB0 => self.cond_branch(ctx, BranchType::Bcs),
            0xB1 => self.lda(ctx, AddressingMode::DirectIndirectIndexedY),
            0xB2 => self.lda(ctx, AddressingMode::DirectIndirect),
            0xB3 => self.lda(ctx, AddressingMode::StackRelativeIndirectIndexed),
            0xB4 => self.ldy(ctx, AddressingMode::DirectX),
            0xB5 => self.lda(ctx, AddressingMode::DirectX),
            0xB6 => self.ldx(ctx, AddressingMode::DirectY),
            0xB7 => self.lda(ctx, AddressingMode::DirectIndirectIndexedLongY),

            0xB9 => self.lda(ctx, AddressingMode::AbsoluteY),
            0xBA => self.tsx(ctx),
            0xBB => self.tyx(ctx),
            0xBC => self.ldy(ctx, AddressingMode::AbsoluteX),
            0xBD => self.lda(ctx, AddressingMode::AbsoluteX),
            0xBE => self.ldx(ctx, AddressingMode::AbsoluteY),
            0xBF => self.lda(ctx, AddressingMode::AbsoluteLongX),

            0xD0 => self.cond_branch(ctx, BranchType::Bne),
            0xD4 => self.pei(ctx),
            0xDA => self.phx(ctx),
            0xDC => self.jmp_far(ctx),

            0xF0 => self.cond_branch(ctx, BranchType::Beq),
            0xF4 => self.pea(ctx),
            0xFA => self.plx(ctx),
            0xFC => self.jsr_aix(ctx),

            _ => unreachable!(),
        }
    }

    // opcode: A8
    fn tay(&mut self, ctx: &mut impl Context) {
        let data = if self.p.x { self.a & 0xFF } else { self.a };
        self.y = data;
        self.set_nz(data);
        ctx.elapse(CPU_CYCLE);
    }

    // opcode: AA
    fn tax(&mut self, ctx: &mut impl Context) {
        let data = if self.p.x { self.a & 0xFF } else { self.a };
        self.x = data;
        self.set_nz(data);
        ctx.elapse(CPU_CYCLE);
    }

    // opcode BA
    fn tsx(&mut self, ctx: &mut impl Context) {
        let data = if self.p.x { self.s & 0xFF } else { self.s };
        self.x = data;
        self.set_nz(data);
        ctx.elapse(CPU_CYCLE);
    }

    // opcode 98
    fn tya(&mut self, ctx: &mut impl Context) {
        let data = if self.p.m { self.y & 0xFF } else { self.y };
        self.a = data;
        self.set_nz(data);
        ctx.elapse(CPU_CYCLE);
    }

    // opcode 8A
    fn txa(&mut self, ctx: &mut impl Context) {
        let data = if self.p.m { self.x & 0xFF } else { self.x };
        self.a = data;
        self.set_nz(data);
        ctx.elapse(CPU_CYCLE);
    }

    // opcode 9A
    fn txs(&mut self, ctx: &mut impl Context) {
        if self.e {
            self.s = self.s & 0xFF00 | self.x & 0xFF;
        } else {
            self.s = self.x;
        }
        ctx.elapse(CPU_CYCLE);
    }

    // opcode 9B
    fn txy(&mut self, ctx: &mut impl Context) {
        let data = if self.p.x { self.x & 0xFF } else { self.x };
        self.y = data;
        self.set_nz(data);
        ctx.elapse(CPU_CYCLE);
    }

    // opcode BB
    fn tyx(&mut self, ctx: &mut impl Context) {
        let data = if self.p.x { self.y & 0xFF } else { self.y };
        self.x = data;
        self.set_nz(data);
        ctx.elapse(CPU_CYCLE);
    }

    // opcode 7B
    fn tdc(&mut self, ctx: &mut impl Context) {
        self.a = self.d;
        self.set_nz(self.a);
        ctx.elapse(CPU_CYCLE);
    }

    // opcode 5B
    fn tcd(&mut self, ctx: &mut impl Context) {
        self.d = self.a;
        self.set_nz(self.d);
        ctx.elapse(CPU_CYCLE);
    }

    // opcode 3B
    fn tsc(&mut self, ctx: &mut impl Context) {
        self.a = self.s;
        self.set_nz(self.a);
        ctx.elapse(CPU_CYCLE);
    }

    // opcode 1B
    fn tcs(&mut self, ctx: &mut impl Context) {
        if self.e {
            self.s = self.s & 0xFF00 | self.a & 0xFF;
        } else {
            self.s = self.a;
        }
        ctx.elapse(CPU_CYCLE);
    }

    // opcode A9
    fn lda_imm(&mut self, ctx: &mut impl Context) {
        if self.is_a_register_8bit() {
            let data = self.fetch_8(ctx);
            self.set_nz(data);
            self.a = data as u16;
        } else {
            let data = self.fetch_16(ctx);
            self.set_nz(data);
            self.a = data;
        }
    }

    fn lda(&mut self, ctx: &mut impl Context, addressing_mode: AddressingMode) {
        let addr = self.get_warp_address(addressing_mode, ctx);
        if self.is_a_register_8bit() {
            let data = addr.read_8(ctx);
            self.set_nz(data);
            self.a = data as u16;
        } else {
            let data = addr.read_16(ctx);
            self.set_nz(data);
            self.a = data;
        }
    }

    fn ldx_imm(&mut self, ctx: &mut impl Context) {
        if self.is_xy_register_8bit() {
            let data = self.fetch_8(ctx);
            self.set_nz(data);
            self.x = data as u16;
        } else {
            let data = self.fetch_16(ctx);
            self.set_nz(data);
            self.x = data;
        }
    }

    fn ldx(&mut self, ctx: &mut impl Context, addressing_mode: AddressingMode) {
        let addr = self.get_warp_address(addressing_mode, ctx);
        if self.is_xy_register_8bit() {
            let data = addr.read_8(ctx);
            self.set_nz(data);
            self.x = data as u16;
        } else {
            let data = addr.read_16(ctx);
            self.set_nz(data);
            self.x = data;
        }
    }

    fn ldy_imm(&mut self, ctx: &mut impl Context) {
        if self.is_xy_register_8bit() {
            let data = self.fetch_8(ctx);
            self.set_nz(data);
            self.y = data as u16;
        } else {
            let data = self.fetch_16(ctx);
            self.set_nz(data);
            self.y = data;
        }
    }

    fn ldy(&mut self, ctx: &mut impl Context, addressing_mode: AddressingMode) {
        let addr = self.get_warp_address(addressing_mode, ctx);
        if self.is_xy_register_8bit() {
            let data = addr.read_8(ctx);
            self.set_nz(data);
            self.y = data as u16;
        } else {
            let data = addr.read_16(ctx);
            self.set_nz(data);
            self.y = data;
        }
    }

    fn stz(&mut self, ctx: &mut impl Context, addressing_mode: AddressingMode) {
        let addr = self.get_warp_address(addressing_mode, ctx);
        if self.is_memory_8bit() {
            addr.write_8(ctx, 0);
        } else {
            addr.write_16(ctx, 0);
        }
    }

    fn sta(&mut self, ctx: &mut impl Context, addressing_mode: AddressingMode) {
        let addr = self.get_warp_address(addressing_mode, ctx);
        if self.is_memory_8bit() {
            addr.write_8(ctx, self.a as u8);
        } else {
            addr.write_16(ctx, self.a);
        }
    }

    fn stx(&mut self, ctx: &mut impl Context, addressing_mode: AddressingMode) {
        let addr = self.get_warp_address(addressing_mode, ctx);
        if self.is_xy_register_8bit() {
            addr.write_8(ctx, self.x as u8);
        } else {
            addr.write_16(ctx, self.x);
        }
    }

    fn sty(&mut self, ctx: &mut impl Context, addressing_mode: AddressingMode) {
        let addr = self.get_warp_address(addressing_mode, ctx);
        if self.is_xy_register_8bit() {
            addr.write_8(ctx, self.y as u8);
        } else {
            addr.write_16(ctx, self.y);
        }
    }

    fn pha(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE);
        if self.is_a_register_8bit() {
            self.push_8(ctx, self.a as u8);
        } else {
            self.push_16(ctx, self.a);
        }
    }

    fn phx(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE);
        if self.is_xy_register_8bit() {
            self.push_8(ctx, self.x as u8);
        } else {
            self.push_16(ctx, self.x);
        }
    }

    fn phy(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE);
        if self.is_xy_register_8bit() {
            self.push_8(ctx, self.y as u8);
        } else {
            self.push_16(ctx, self.y);
        }
    }

    fn php(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE);
        self.push_8(ctx, self.p.into());
    }

    fn phb(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE);
        self.push_8(ctx, self.db);
    }

    fn phk(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE);
        self.push_8(ctx, self.pb);
    }

    fn phd(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE);
        self.push_16(ctx, self.d);
    }

    fn pei(&mut self, ctx: &mut impl Context) {
        let addr = self.get_warp_address(AddressingMode::Direct, ctx);
        let data = addr.read_16(ctx);
        self.push_16(ctx, data);
    }

    fn pea(&mut self, ctx: &mut impl Context) {
        let data = self.fetch_16(ctx);
        self.push_16(ctx, data);
    }

    fn per(&mut self, ctx: &mut impl Context) {
        let disp = self.fetch_16(ctx);
        ctx.elapse(CPU_CYCLE);
        self.push_16(ctx, self.pc.wrapping_add(disp));
    }

    fn pla(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE * 2);
        if self.is_a_register_8bit() {
            self.a = self.pop_8(ctx) as u16;
        } else {
            self.a = self.pop_16(ctx);
        }
        self.set_nz(self.a);
    }

    fn plx(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE * 2);
        if self.is_xy_register_8bit() {
            self.x = self.pop_8(ctx) as u16;
        } else {
            self.x = self.pop_16(ctx);
        }
        self.set_nz(self.x);
    }

    fn ply(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE * 2);
        if self.is_xy_register_8bit() {
            self.y = self.pop_8(ctx) as u16;
        } else {
            self.y = self.pop_16(ctx);
        }
        self.set_nz(self.y);
    }

    fn pld(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE * 2);
        self.d = self.pop_16(ctx);
        self.set_nz(self.d);
    }

    fn plb(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE * 2);
        self.db = self.pop_8(ctx);
        self.set_nz(self.db);
    }

    fn plp(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE * 2);
        self.p = self.pop_8(ctx).into();
    }

    fn alu(&mut self, ctx: &mut impl Context, alu_type: AluType, addressing_mode: AddressingMode) {
        if self.is_a_register_8bit() {
            let a = self.a as u8;
            let b = self.get_warp_address(addressing_mode, ctx).read_8(ctx);
            // TODO
            // let c = match alu_type {
            //     AluType::Or => a | b,
            //     AluType::And => a & b,
            //     AluType::Xor => a ^ b,
            //     AluType::Add => a.wrapping_add(b),
            //     AluType::Sub => a.wrapping_sub(b),
            //     AluType::Cmp => {
            //         let result = a.wrapping_sub(b);
            //         self.p.c = a >= b;
            //         self.set_nz(result);
            //         return;
            //     }
            // };
            // self.set_nz(c);
        }
    }

    fn asl_a(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE);
        if self.is_a_register_8bit() {
            let data = self.a as u8;
            self.p.c = (data >> 7) & 1 == 1;
            let result = data << 1;
            self.set_nz(result);
            self.a = result as u16;
        } else {
            let data = self.a;
            self.p.c = (data >> 15) & 1 == 1;
            let result = data << 1;
            self.set_nz(result);
            self.a = result;
        }
    }

    fn asl_with_addressing(&mut self, ctx: &mut impl Context, addressing_mode: AddressingMode) {
        ctx.elapse(CPU_CYCLE);
        let addr = self.get_warp_address(addressing_mode, ctx);
        if self.is_memory_8bit() {
            let data = addr.read_8(ctx);
            self.p.c = (data >> 7) & 1 == 1;
            let result = data << 1;
            self.set_nz(result);
            addr.write_8(ctx, result);
        } else {
            let data = addr.read_16(ctx);
            self.p.c = (data >> 15) & 1 == 1;
            let result = data << 1;
            self.set_nz(result);
            addr.write_16(ctx, result);
        }
    }

    fn lsr_a(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE);
        if self.is_a_register_8bit() {
            let data = self.a as u8;
            self.p.c = data & 1 == 1;
            let result = data >> 1;
            self.set_nz(result);
            self.a = result as u16;
        } else {
            let data = self.a;
            self.p.c = data & 1 == 1;
            let result = data >> 1;
            self.set_nz(result);
            self.a = result;
        }
    }

    fn lsr_with_addressing(&mut self, ctx: &mut impl Context, addressing_mode: AddressingMode) {
        ctx.elapse(CPU_CYCLE);
        let addr = self.get_warp_address(addressing_mode, ctx);
        if self.is_memory_8bit() {
            let data = addr.read_8(ctx);
            self.p.c = data & 1 == 1;
            let result = data >> 1;
            self.set_nz(result);
            addr.write_8(ctx, result);
        } else {
            let data = addr.read_16(ctx);
            self.p.c = data & 1 == 1;
            let result = data >> 1;
            self.set_nz(result);
            addr.write_16(ctx, result);
        }
    }

    fn rol_a(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE);
        if self.is_a_register_8bit() {
            let data = self.a as u8;
            let c = self.p.c as u8;
            self.p.c = (data >> 7) & 1 == 1;
            let result = (data << 1) | c;
            self.set_nz(result);
            self.a = result as u16;
        } else {
            let data = self.a;
            let c = self.p.c as u16;
            self.p.c = (data >> 15) & 1 == 1;
            let result = (data << 1) | c;
            self.set_nz(result);
            self.a = result;
        }
    }

    fn rol_with_addressing(&mut self, ctx: &mut impl Context, addressing_mode: AddressingMode) {
        ctx.elapse(CPU_CYCLE);
        let addr = self.get_warp_address(addressing_mode, ctx);
        if self.is_memory_8bit() {
            let data = addr.read_8(ctx);
            let c = self.p.c as u8;
            self.p.c = (data >> 7) & 1 == 1;
            let result = (data << 1) | c;
            self.set_nz(result);
            addr.write_8(ctx, result);
        } else {
            let data = addr.read_16(ctx);
            let c = self.p.c as u16;
            self.p.c = (data >> 15) & 1 == 1;
            let result = (data << 1) | c;
            self.set_nz(result);
            addr.write_16(ctx, result);
        }
    }

    fn ror_a(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE);
        if self.is_a_register_8bit() {
            let data = self.a as u8;
            let c = self.p.c as u8;
            self.p.c = data & 1 == 1;
            let result = (data >> 1) | (c << 7);
            self.set_nz(result);
            self.a = result as u16;
        } else {
            let data = self.a;
            let c = self.p.c as u16;
            self.p.c = data & 1 == 1;
            let result = (data >> 1) | (c << 15);
            self.set_nz(result);
            self.a = result;
        }
    }

    fn ror_with_addressing(&mut self, ctx: &mut impl Context, addressing_mode: AddressingMode) {
        ctx.elapse(CPU_CYCLE);
        let addr = self.get_warp_address(addressing_mode, ctx);
        if self.is_memory_8bit() {
            let data = addr.read_8(ctx);
            let c = self.p.c as u8;
            self.p.c = data & 1 == 1;
            let result = (data >> 1) | (c << 7);
            self.set_nz(result);
            addr.write_8(ctx, result);
        } else {
            let data = addr.read_16(ctx);
            let c = self.p.c as u16;
            self.p.c = data & 1 == 1;
            let result = (data >> 1) | (c << 15);
            self.set_nz(result);
            addr.write_16(ctx, result);
        }
    }

    fn jmp_disp_8(&mut self, ctx: &mut impl Context) {
        let disp = self.fetch_8(ctx) as i8 as u16;
        ctx.elapse(CPU_CYCLE);
        if self.e && (self.pc & 0xFF) + (disp & 0xFF) >= 0x100 {
            ctx.elapse(CPU_CYCLE);
        }
        self.pc = self.pc.wrapping_add(disp);
    }

    fn jmp_disp_16(&mut self, ctx: &mut impl Context) {
        let disp = self.fetch_16(ctx);
        ctx.elapse(CPU_CYCLE);
        self.pc = self.pc.wrapping_add(disp);
    }

    fn jmp_abs(&mut self, ctx: &mut impl Context) {
        let addr = self.fetch_16(ctx);
        self.pc = addr;
    }

    fn jmp_abs_long(&mut self, ctx: &mut impl Context) {
        let addr = self.fetch_24(ctx);
        self.pc = addr as u16;
        self.pb = (addr >> 16) as u8;
    }

    fn jmp_nnnn(&mut self, ctx: &mut impl Context) {
        let addr = WarpAddress {
            addr: self.fetch_16(ctx) as u32,
            mode: WarpMode::Warp16bit,
        }
        .read_16(ctx);
        self.pc = addr;
    }

    fn jmp_nnnn_x(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE);
        let addr = WarpAddress {
            addr: (self.pb as u32) << 16 | self.fetch_16(ctx) as u32,
            mode: WarpMode::Warp16bit,
        }
        .offset(self.x)
        .read_16(ctx);

        self.pc = addr;
    }

    fn jmp_far(&mut self, ctx: &mut impl Context) {
        let addr = WarpAddress {
            addr: self.fetch_16(ctx) as u32,
            mode: WarpMode::Warp16bit,
        }
        .read_24(ctx);
        self.pc = addr as u16;
        self.pb = (addr >> 16) as u8;
    }

    fn jsr_abs(&mut self, ctx: &mut impl Context) {
        let addr = self.fetch_16(ctx);
        ctx.elapse(CPU_CYCLE);
        self.push_16(ctx, self.pc.wrapping_sub(1));
        self.pc = addr;
    }

    fn jsl_far(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE);
        let pc = self.fetch_16(ctx);
        let pb = self.fetch_8(ctx);
        self.push_8(ctx, self.pb);
        self.push_16(ctx, self.pc.wrapping_sub(1));
        self.pc = pc;
        self.pb = pb;
    }

    fn jsr_aix(&mut self, ctx: &mut impl Context) {
        let addr = self
            .get_warp_address(AddressingMode::AbsoluteIndexedIndirect, ctx)
            .read_16(ctx);
        ctx.elapse(CPU_CYCLE);
        self.push_16(ctx, self.pc.wrapping_sub(1));
        self.pc = addr;
    }

    fn rti(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE);
        self.p = self.pop_8(ctx).into();
        self.pc = self.pop_16(ctx);
        if !self.e {
            self.pb = self.pop_8(ctx);
            ctx.elapse(CPU_CYCLE);
        }
    }

    fn rtl(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE * 2);
        self.pc = self.pop_16(ctx).wrapping_add(1);
        self.pb = self.pop_8(ctx);
    }

    fn rts(&mut self, ctx: &mut impl Context) {
        ctx.elapse(CPU_CYCLE * 3);
        self.pc = self.pop_16(ctx).wrapping_add(1);
    }

    fn cond_branch(&mut self, ctx: &mut impl Context, condition: BranchType) {
        let disp = self.fetch_8(ctx) as i8 as u16;
        if self.check_branch_condition(condition) {
            ctx.elapse(CPU_CYCLE);
            let prev_pc = self.pc;
            self.pc = self.pc.wrapping_add(disp);
            if self.e && prev_pc & 0xFF00 != self.pc & 0xFF00 {
                ctx.elapse(CPU_CYCLE);
            }
        }
    }

    fn check_branch_condition(&self, condition: BranchType) -> bool {
        match condition {
            BranchType::Bpl => !self.p.n,
            BranchType::Bmi => self.p.n,
            BranchType::Bvc => !self.p.v,
            BranchType::Bvs => self.p.v,
            BranchType::Bcc => !self.p.c,
            BranchType::Bcs => self.p.c,
            BranchType::Bne => !self.p.z,
            BranchType::Beq => self.p.z,
        }
    }
}
