#[derive(Default)]
pub struct Interrupt {
    // Nmi
    nmi_flag: bool,
    nmi_enable: bool,
    nmi: bool,

    // irq
    hv_irq_enable: u8, // 0x4200.4-5 0=Disable, 1=At H=H + V=Any, 2=At V=V + H=0, 3=At H=H + V=V

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
        let prev = self.nmi & self.nmi_enable;
        self.nmi_flag = flag;
        if !prev && self.nmi_enable && self.nmi_enable {
            self.nmi = true;
        }
    }

    pub fn set_nmi_enable(&mut self, flag: bool) {
        let prev = self.nmi & self.nmi_enable;
        self.nmi_enable = flag;
        if !prev && self.nmi_enable && self.nmi_enable {
            self.nmi = true;
        }
    }

    pub fn nmi_occurred(&self) -> bool {
        self.nmi
    }

    pub fn set_hv_irq_enable(&mut self, val: u8) {
        self.hv_irq_enable = val;
    }

    pub fn get_hv_irq_enable(&self) -> u8 {
        self.hv_irq_enable
    }

    pub fn set_joypad_enable(&mut self, flag: bool) {
        self.joypad_enable = flag;
    }
}
