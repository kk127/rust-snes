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
    pd: u8,
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
            pd: 0,
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

impl Cpu {
    fn fetch_8(&mut self, context: &mut impl Context) -> u8 {
        let data = context.bus_read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        data
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
