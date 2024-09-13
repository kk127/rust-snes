use log::info;

pub struct Cartridge {
    rom: Rom,
    sram: Vec<u8>,
}

impl Cartridge {
    pub fn new(rom: Vec<u8>) -> Cartridge {
        let rom = Rom::from_bytes(&rom).expect("Failed to parse ROM");
        let sram = vec![0; rom.header.ram_size * 1024];
        Cartridge { rom, sram }
    }
}

impl Cartridge {
    pub fn read(&self, addr: u32) -> u8 {
        match self.rom.header.map_mode {
            MapMode::LoRom => {
                let bank = (addr >> 16) as usize;
                let offset = (addr & 0xFFFF) as usize;
                match bank {
                    0x00..=0x7D => self.read(addr + 0x800000),
                    0x7E..=0x7F => unreachable!(),
                    0x80..=0xFF => match offset {
                        0x0000..=0x7FFF => match bank {
                            0x80..=0xBF => {
                                unreachable!("Invalid bank: {:02X}, offset: {:04X}", bank, offset);
                            }
                            0xC0..=0xEF => self.read(addr + 0x8000),
                            0xF0..=0xFF => {
                                let sram_offset = (bank - 0xF0) * 1024 * 32 + offset;
                                let sram_index = sram_offset % self.sram.len();
                                self.sram[sram_index]
                            }
                            _ => unreachable!(),
                        },
                        0x8000..=0xFFFF => {
                            let rom_offset = (bank - 0x80) * 1024 * 32 + (offset - 0x8000);
                            let rom_index = rom_offset % self.rom.rom.len();
                            self.rom.rom[rom_index]
                        }
                        _ => unreachable!(),
                    },

                    _ => unreachable!(),
                }
            }
            MapMode::HiRom => {
                let bank = (addr >> 16) as usize;
                let offset = (addr & 0xFFFF) as usize;
                match bank {
                    0x00..=0x3F => match offset {
                        0x0000..=0x5FFF => unreachable!(),
                        0x6000..=0x7FFF => {
                            let sram_offset = bank * 1024 * 8 + (offset - 0x6000);
                            let sram_index = sram_offset % self.sram.len();
                            self.sram[sram_index]
                        }
                        0x8000..=0xFFFF => {
                            let rom_index = (addr as usize) % self.rom.rom.len();
                            self.rom.rom[rom_index]
                        }
                        _ => unreachable!(),
                    },
                    0x40..=0x7D => {
                        let rom_index = (addr as usize - 0x400000) % self.rom.rom.len();
                        self.rom.rom[rom_index]
                    }
                    0x80..=0xBF => match offset {
                        0x0000..=0x5FFF => unreachable!(),
                        0x6000..=0x7FFF => {
                            let sram_offset = (bank - 0x80) * 1024 * 8 + (offset - 0x6000);
                            let sram_index = sram_offset % self.sram.len();
                            self.sram[sram_index]
                        }
                        0x8000..=0xFFFF => {
                            let rom_index = (addr as usize - 0x800000) % self.rom.rom.len();
                            self.rom.rom[rom_index]
                        }
                        _ => unreachable!(),
                    },
                    0xC0..=0xFF => {
                        let rom_index = (addr as usize - 0xC00000) % self.rom.rom.len();
                        self.rom.rom[rom_index]
                    }
                    _ => unreachable!(),
                }
            }
            _ => unimplemented!(),
        }
    }

    pub fn write(&mut self, addr: u32, data: u8) {
        match self.rom.header.map_mode {
            MapMode::LoRom => {
                let bank = (addr >> 16) as usize;
                let offset = (addr & 0xFFFF) as usize;
                match bank {
                    0x00..=0x7D => self.write(addr + 0x800000, data),
                    0x7E..=0x7F => unreachable!(),
                    0x80..=0xFF => match offset {
                        0x0000..=0x7FFF => match bank {
                            0x80..=0xBF => unreachable!(),
                            0xC0..=0xEF => self.write(addr + 0x8000, data),
                            0xF0..=0xFF => {
                                let sram_offset = (bank - 0xF0) * 1024 * 32 + offset;
                                let sram_index = sram_offset % self.sram.len();
                                self.sram[sram_index] = data;
                            }
                            _ => unreachable!(),
                        },
                        0x8000..=0xFFFF => {
                            let rom_offset = (bank - 0x80) * 1024 * 32 + (offset - 0x8000);
                            let rom_index = rom_offset % self.rom.rom.len();
                            self.rom.rom[rom_index] = data;
                        }
                        _ => unreachable!(),
                    },

                    _ => unreachable!(),
                }
            }
            MapMode::HiRom => {
                let bank = (addr >> 16) as usize;
                let offset = (addr & 0xFFFF) as usize;
                match bank {
                    0x00..=0x3F => match offset {
                        0x0000..=0x5FFF => unreachable!(),
                        0x6000..=0x7FFF => {
                            let sram_offset = bank * 1024 * 8 + (offset - 0x6000);
                            let sram_index = sram_offset % self.sram.len();
                            self.sram[sram_index] = data;
                        }
                        0x8000..=0xFFFF => {
                            let rom_index = (addr as usize) % self.rom.rom.len();
                            self.rom.rom[rom_index] = data;
                        }
                        _ => unreachable!(),
                    },
                    0x40..=0x7D => {
                        let rom_index = (addr as usize - 0x400000) % self.rom.rom.len();
                        self.rom.rom[rom_index] = data;
                    }
                    0x80..=0xBF => match offset {
                        0x0000..=0x5FFF => unreachable!(),
                        0x6000..=0x7FFF => {
                            let sram_offset = (bank - 0x80) * 1024 * 8 + (offset - 0x6000);
                            let sram_index = sram_offset % self.sram.len();
                            self.sram[sram_index] = data;
                        }
                        0x8000..=0xFFFF => {
                            let rom_index = (addr as usize - 0x800000) % self.rom.rom.len();
                            self.rom.rom[rom_index] = data;
                        }
                        _ => unreachable!(),
                    },
                    0xC0..=0xFF => {
                        let rom_index = (addr as usize - 0xC00000) % self.rom.rom.len();
                        self.rom.rom[rom_index] = data;
                    }
                    _ => unreachable!(),
                }
            }
            _ => unimplemented!(),
        }
    }
}

struct Rom {
    header: Header,
    rom: Vec<u8>,
}

impl Rom {
    fn from_bytes(bytes: &[u8]) -> Result<Rom, String> {
        for &base in [0x007F00, 0x00FF00, 0x40FF00].iter() {
            if base + 0x100 > bytes.len() {
                continue;
            }

            if let Ok(header) = parse_header(bytes, base) {
                info!("ROM title: {}", header.title);
                info!("ROM speed: {:?}", header.speed);
                info!("ROM map mode: {:?}", header.map_mode);
                info!("ROM chipset: {:02X}", header.chipset);
                info!("ROM size: {}KB", header.rom_size);
                info!("RAM size: {}KB", header.ram_size);
                info!("Country: {:02X}", header.country);
                info!("Developer ID: {:02X}", header.developer_id);
                info!("ROM version: {:02X}", header.rom_version);
                info!("Checksum complement: {:04X}", header.checksum_complement);
                info!("Checksum: {:04X}", header.checksum);

                return Ok(Rom {
                    header,
                    rom: bytes.to_vec(),
                });
            }
        }
        Err("Failed to parse ROM".to_string())
    }
}

fn parse_header(bytes: &[u8], base: usize) -> Result<Header, String> {
    let checksum_complement =
        u16::from_le_bytes(bytes[base + 0xDC..base + 0xDC + 2].try_into().unwrap());
    let checksum = u16::from_le_bytes(bytes[base + 0xDE..base + 0xDE + 2].try_into().unwrap());
    // TODO: Commnet out for CPUADC test
    if checksum_complement != !checksum {
        return Err("Checksum error".to_string());
    }

    let title = match std::str::from_utf8(&bytes[base + 0xC0..base + 0xC0 + 21]) {
        Ok(title) => title.trim().to_string(),
        Err(_) => "Invalid Title".to_string(),
    };

    let speed = Speed::from((bytes[base + 0xD5] >> 4) & 1);
    let map_mode = MapMode::from(bytes[base + 0xD5] & 0xF);

    let chipset = bytes[base + 0xD6];

    let rom_size = 1 << bytes[base + 0xD7] as usize;

    let ram_size = match bytes[base + 0xD8] {
        0 => 0,
        n => 1 << n as usize,
    };

    let country = bytes[base + 0xD9];

    let developer_id = bytes[base + 0xDA];

    let rom_version = bytes[base + 0xDB];

    Ok(Header {
        title,
        speed,
        map_mode,
        chipset,
        rom_size,
        ram_size,
        country,
        developer_id,
        rom_version,
        checksum_complement,
        checksum,
    })
}

struct Header {
    title: String,
    speed: Speed,
    map_mode: MapMode,
    chipset: u8,
    rom_size: usize,
    ram_size: usize,
    country: u8,
    developer_id: u8,
    rom_version: u8,
    checksum_complement: u16,
    checksum: u16,
}

#[derive(Debug)]
enum Speed {
    Slow,
    Fast,
}

impl From<u8> for Speed {
    fn from(val: u8) -> Speed {
        match val {
            0 => Speed::Slow,
            1 => Speed::Fast,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
enum MapMode {
    LoRom,
    HiRom,
    SDd1,
    SA1,
    ExHiRom,
    Spc7110,
}

impl From<u8> for MapMode {
    fn from(val: u8) -> MapMode {
        match val {
            0 => MapMode::LoRom,
            1 => MapMode::HiRom,
            2 => MapMode::SDd1,
            3 => MapMode::SA1,
            4 => MapMode::ExHiRom,
            5 => MapMode::Spc7110,
            _ => unreachable!("Unknown map mode: {}", val),
        }
    }
}
