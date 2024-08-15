pub struct Cartridge {
    rom: Vec<u8>,
}

impl Cartridge {
    pub fn new(rom: Vec<u8>) -> Cartridge {
        Cartridge { rom }
    }
}

impl Cartridge {
    pub fn read(&self, addr: u32) -> u8 {
        let bank = addr >> 16;
        // TODO
        let mut addr = (addr as u16) as usize;
        // println!("bank: {:x}, addr: {:x}", bank, addr);
        // 32KiBのときはミラー
        if addr >= 0x8000 {
            addr -= 0x8000;
        }
        // println!("after addr: {:x}", addr);

        let val = match bank {
            0x00..=0x3F => self.rom[addr as usize],
            0x40..=0x7D => {
                let index = addr as usize - 0x400000;
                self.rom[index]
            }
            0x80..=0xBF => {
                let index = addr as usize - 0x800000;
                self.rom[index]
            }
            0xC0..=0xFF => {
                let index = addr as usize - 0xC00000;
                self.rom[index]
            }
            _ => unreachable!(),
        };
        // println!("val: {:x}", val);
        val
    }

    pub fn write(&mut self, addr: u32, data: u8) {
        let bank = addr >> 16;
        match bank {
            0x00..=0x3F => self.rom[addr as usize] = data,
            0x40..=0x7D => {
                let index = addr as usize - 0x400000;
                self.rom[index] = data;
            }
            0x80..=0xBF => {
                let index = addr as usize - 0x800000;
                self.rom[index] = data;
            }
            0xC0..=0xFF => {
                let index = addr as usize - 0xC00000;
                self.rom[index] = data;
            }
            _ => unreachable!(),
        }
    }
}
