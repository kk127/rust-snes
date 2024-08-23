use log::info;

pub struct Cartridge {
    rom: Rom,
}

impl Cartridge {
    pub fn new(rom: Vec<u8>) -> Cartridge {
        let rom = Rom::from_bytes(&rom).expect("Failed to parse ROM");
        Cartridge { rom }
    }
}

impl Cartridge {
    pub fn read(&self, addr: u32) -> u8 {
        match self.rom.header.map_mode {
            MapMode::LoRom => {
                // let offset = (addr as usize >> 16) & 0x7;
                // let bank_num_per_rom_size = self.rom.header.rom_size / 32;
                // let rom_offset = offset % bank_num_per_rom_size;
                // let page_offset = (addr & 0x00FFFF) as usize - 0x8000;
                // let rom_addr = rom_offset * 0x8000 + page_offset;
                let rom_addr = ((addr & 0x00FFFF) as usize - 0x8000) % self.rom.rom.len();
                self.rom.rom[rom_addr]
            }
            _ => unimplemented!(),
        }
    }

    pub fn write(&mut self, addr: u32, data: u8) {
        let bank = addr >> 16;
        match bank {
            0x00..=0x3F => self.rom.rom[addr as usize - 0x008000] = data,
            0x40..=0x7D => {
                let index = addr as usize - 0x400000;
                self.rom.rom[index] = data;
            }
            0x80..=0xBF => {
                let index = addr as usize - 0x800000;
                self.rom.rom[index] = data;
            }
            0xC0..=0xFF => {
                let index = addr as usize - 0xC00000;
                self.rom.rom[index] = data;
            }
            _ => unreachable!(),
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
    // if checksum_complement != !checksum {
    //     return Err("Checksum error".to_string());
    // }

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
            _ => unreachable!(),
        }
    }
}
