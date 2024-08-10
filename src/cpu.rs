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

    fn write8(&self, context: &mut impl Context, data: u8) {
        context.bus_write(self.unwrap(), data);
    }

    fn write16(&self, context: &mut impl Context, data: u16) {
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

    fn get_warp_address(
        &mut self,
        adressing_mode: AddressingMode,
        ctx: &mut impl Context,
    ) -> WarpAddress {
        match adressing_mode {
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
            // AbsoluteIndexedIndirect
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

    fn excecute_instruction(&mut self, context: &mut impl Context) {
        let opcode = self.fetch_8(context);
        match opcode {
            0x1B => self.tcs(context),

            0x3B => self.tsc(context),

            0x5B => self.tcd(context),

            0x7B => self.tdc(context),

            0x8A => self.txa(context),

            0x98 => self.tya(context),
            0x9A => self.txs(context),
            0x9B => self.txy(context),

            0xA8 => self.tay(context),
            0xAA => self.tax(context),

            0xBA => self.tsx(context),
            0xBB => self.tyx(context),

            _ => unreachable!(),
        }
    }

    // opcode: A8
    fn tay(&mut self, context: &mut impl Context) {
        let data = if self.p.x { self.a & 0xFF } else { self.a };
        self.y = data;
        self.set_nz(data);
        context.elapse(CPU_CYCLE);
    }

    // opcode: AA
    fn tax(&mut self, context: &mut impl Context) {
        let data = if self.p.x { self.a & 0xFF } else { self.a };
        self.x = data;
        self.set_nz(data);
        context.elapse(CPU_CYCLE);
    }

    // opcode BA
    fn tsx(&mut self, context: &mut impl Context) {
        let data = if self.p.x { self.s & 0xFF } else { self.s };
        self.x = data;
        self.set_nz(data);
        context.elapse(CPU_CYCLE);
    }

    // opcode 98
    fn tya(&mut self, context: &mut impl Context) {
        let data = if self.p.m { self.y & 0xFF } else { self.y };
        self.a = data;
        self.set_nz(data);
        context.elapse(CPU_CYCLE);
    }

    // opcode 8A
    fn txa(&mut self, context: &mut impl Context) {
        let data = if self.p.m { self.x & 0xFF } else { self.x };
        self.a = data;
        self.set_nz(data);
        context.elapse(CPU_CYCLE);
    }

    // opcode 9A
    fn txs(&mut self, context: &mut impl Context) {
        if self.e {
            self.s = self.s & 0xFF00 | self.x & 0xFF;
        } else {
            self.s = self.x;
        }
        context.elapse(CPU_CYCLE);
    }

    // opcode 9B
    fn txy(&mut self, context: &mut impl Context) {
        let data = if self.p.x { self.x & 0xFF } else { self.x };
        self.y = data;
        self.set_nz(data);
        context.elapse(CPU_CYCLE);
    }

    // opcode BB
    fn tyx(&mut self, context: &mut impl Context) {
        let data = if self.p.x { self.y & 0xFF } else { self.y };
        self.x = data;
        self.set_nz(data);
        context.elapse(CPU_CYCLE);
    }

    // opcode 7B
    fn tdc(&mut self, context: &mut impl Context) {
        self.a = self.d;
        self.set_nz(self.a);
        context.elapse(CPU_CYCLE);
    }

    // opcode 5B
    fn tcd(&mut self, context: &mut impl Context) {
        self.d = self.a;
        self.set_nz(self.d);
        context.elapse(CPU_CYCLE);
    }

    // opcode 3B
    fn tsc(&mut self, context: &mut impl Context) {
        self.a = self.s;
        self.set_nz(self.a);
        context.elapse(CPU_CYCLE);
    }

    // opcode 1B
    fn tcs(&mut self, context: &mut impl Context) {
        if self.e {
            self.s = self.s & 0xFF00 | self.a & 0xFF;
        } else {
            self.s = self.a;
        }
        context.elapse(CPU_CYCLE);
    }
}
