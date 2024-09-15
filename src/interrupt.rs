#[derive(Default)]
pub struct Interrupt {
    // Nmi
    nmi_flag: bool,
    nmi_enable: bool,
    nmi: bool,
    nmi_raise: bool,

    // irq
    hv_irq_enable: u8, // 0x4200.4-5 0=Disable, 1=At H=H + V=Any, 2=At V=V + H=0, 3=At H=H + V=V
    h_count: u16,      // 0x4207, 0x4208
    v_count: u16,      // 0x4209, 0x420A
    irq: bool,

    // JoyPad
    joypad_enable: bool,
}

impl Interrupt {
    pub fn get_nmi_flag(&mut self) -> bool {
        let ret = self.nmi_flag;
        self.nmi_flag = false;
        ret
    }

    pub fn set_nmi_flag(&mut self, flag: bool) {
        let prev = self.nmi_flag & self.nmi_enable;
        self.nmi_flag = flag;
        if !prev && self.nmi_enable && self.nmi_flag {
            self.nmi_raise = true;
        }
    }

    pub fn set_nmi_enable(&mut self, flag: bool) {
        let prev = self.nmi_flag & self.nmi_enable;
        self.nmi_enable = flag;
        if !prev && self.nmi_enable && self.nmi_flag {
            self.nmi_raise = true;
        }
    }

    pub fn nmi_occurred(&mut self) -> bool {
        let ret = self.nmi;
        self.nmi = self.nmi_raise;
        self.nmi_raise = false;
        ret
    }

    pub fn set_hv_irq_enable(&mut self, val: u8) {
        self.hv_irq_enable = val;
        if val == 0 {
            self.irq = false;
        }
    }

    pub fn get_hv_irq_enable(&self) -> u8 {
        self.hv_irq_enable
    }

    pub fn set_irq(&mut self, flag: bool) {
        self.irq = flag;
    }

    pub fn irq_occurred(&self) -> bool {
        self.irq
    }

    pub fn set_h_count(&mut self, val: u16) {
        self.h_count = val;
    }

    pub fn get_h_count(&self) -> u16 {
        self.h_count
    }

    pub fn set_v_count(&mut self, val: u16) {
        self.v_count = val;
    }

    pub fn get_v_count(&self) -> u16 {
        self.v_count
    }
}
