use crate::context;
use modular_bitfield::prelude::*;

use log::debug;
trait Context: context::Timing {}
impl<T: context::Timing> Context for T {}

pub struct Ppu {
    pub screen: [u16; 256 * 224],
    pub frame_number: u64,
    counter: u64,

    x: u16,
    y: u16,

    vram: [u8; 0x10000], // 64KB
    cgram: [u16; 0x100], // 512B

    // Ppu control registers
    display_control_1: u8,      // $2100
    display_control_2: u8,      // $2133
    main_screen_desination: u8, // $212C
    sub_screen_destination: u8, // $212D

    // BG control registers
    bg_mode_and_character_size: u8,                 // $2105
    mosaic_size_and_enable: u8,                     // $2106
    bg_screen_base_and_size: [BGScreenBaseSize; 4], // $2107, $2108, $2109, $210A
    bg_character_base: u16,                         // $210B $210C
    bg_hofs: [u8; 4],                               // $210D, $210F, $2111, $2113
    bg_vofs: [u8; 4],                               // $210E, $2110, $2112, $2114

    // I/O port registers
    vram_mode: VramAddrIncMode, // $2115
    vram_addr: u16,             // $2116, $2117
    palette_cgram_addr: u16,    // $2121
    palette_cgram_low_data: u8,
    // Rotational and scaling registers
    // TODO

    // Sprite control registers
    // TODO

    // Window control registers
    // TODO

    // Color math registers
    // TODO
}

#[bitfield(bits = 8)]
#[derive(Debug, Default)]
struct VramAddrIncMode {
    increment_step: B2,
    transration: B2,
    #[skip]
    __: B3,
    is_incremet_after_high_bit: bool,
}

impl VramAddrIncMode {
    fn get_inc(&self) -> u16 {
        match self.increment_step() {
            0 => 1,
            1 => 32,
            2 => 128,
            3 => 128,
            _ => unreachable!(),
        }
    }

    fn get_transration(&self, addr: u16) -> u16 {
        match self.transration() {
            0 => addr,
            1 => addr & 0xFF00 | (addr & 0x001F) << 3 | (addr & 0x00E0) >> 5,
            2 => addr & 0xFE00 | (addr & 0x003F) << 3 | (addr & 0x01C0) >> 6,
            3 => addr & 0xFC00 | (addr & 0x007F) << 3 | (addr & 0x0380) >> 7,
            _ => unreachable!(),
        }
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Ppu {
            screen: [0; 256 * 224],
            frame_number: 0,
            counter: 0,
            x: 0,
            y: 0,
            vram: [0; 0x10000],
            cgram: [0; 0x100],
            display_control_1: 0,
            display_control_2: 0,
            main_screen_desination: 0,
            sub_screen_destination: 0,
            bg_mode_and_character_size: 0,
            mosaic_size_and_enable: 0,
            bg_screen_base_and_size: [BGScreenBaseSize::new(); 4],
            bg_character_base: 0,
            bg_hofs: [0; 4],
            bg_vofs: [0; 4],
            vram_mode: Default::default(), // 別の型はDefault::default()を使っても良い
            vram_addr: 0,
            palette_cgram_addr: 0,
            palette_cgram_low_data: 0,
        }
    }
}

impl Ppu {
    pub fn read(&mut self, addr: u16, ctx: &mut impl Context) -> u8 {
        println!("ppu read: {:x}", addr);
        0
    }

    pub fn write(&mut self, addr: u16, data: u8, ctx: &mut impl Context) {
        match addr {
            0x2100 => self.display_control_1 = data,
            0x2105 => self.bg_mode_and_character_size = data,
            0x2106 => self.mosaic_size_and_enable = data,
            0x2107 | 0x2108 | 0x2109 | 0x210A => {
                let index = (addr - 0x2107) as usize;
                self.bg_screen_base_and_size[index].bytes[0] = data;
            }

            0x210B | 0x210C => {
                let index = (addr - 0x210B) as usize;
                self.bg_character_base = self.bg_character_base | (data as u16) << (8 * index);
            }
            0x210D | 0x210F | 0x2111 | 0x2113 => {
                let index = (addr - 0x210D) as usize / 2;
                self.bg_hofs[index] = data;
            }
            0x210E | 0x2110 | 0x2112 | 0x2114 => {
                let index = (addr - 0x210E) as usize / 2;
                self.bg_vofs[index] = data;
            }

            0x2115 => self.vram_mode.bytes[0] = data,
            0x2116 => self.vram_addr = self.vram_addr & 0x7F00 | data as u16,
            0x2117 => self.vram_addr = self.vram_addr & 0x00FF | (data as u16) << 8,
            0x2118 | 0x2119 => {
                let offset = addr - 0x2118;
                let vram_addr = self.vram_mode.get_transration(self.vram_addr) * 2 + offset;
                // debug!("vram_addr: 0x{:x}, data: 0x{:x}", vram_addr, data);
                self.vram[vram_addr as usize] = data;
                if self.vram_mode.is_incremet_after_high_bit() == (offset == 1) {
                    self.vram_addr = (self.vram_addr + self.vram_mode.get_inc()) & 0x7FFF;
                }
            }
            0x2121 => self.palette_cgram_addr = data as u16 * 2,
            0x2122 => {
                if self.palette_cgram_addr & 1 == 0 {
                    self.palette_cgram_low_data = data;
                } else {
                    self.cgram[self.palette_cgram_addr as usize / 2] =
                        (data as u16) << 8 | self.palette_cgram_low_data as u16;
                }
                self.palette_cgram_addr = (self.palette_cgram_addr + 1) & 0x1FF;
            }

            0x212C => self.main_screen_desination = data,
            0x212D => self.sub_screen_destination = data,

            0x2133 => self.display_control_2 = data,

            _ => {
                println!("Write unimplemeted, addr: {:x}, data: {:x}", addr, data);
            }
        }
    }

    pub fn tick(&mut self, ctx: &mut impl Context) {
        // println!("cgram: {:?}", self.cgram);
        loop {
            if self.counter + 4 > ctx.now() {
                break;
            }
            // println!("ppu tick");
            // println!("frame_number: {}", self.frame_number);
            // println!("x: {}, y: {}", self.x, self.y);

            self.counter += 4;

            self.x += 1;
            if self.x == 340 {
                self.x = 0;
                self.y += 1;
                if self.y == 262 {
                    self.y = 0;
                    self.frame_number += 1;
                }
            }
            if self.x == 22 && (1..225).contains(&self.y) {
                self.render_line(self.y - 1);
            }
        }
    }

    fn render_line(&mut self, y: u16) {
        for x in 0..256 {
            self.screen[(y * 256 + x) as usize] = self.get_xy_color(x, y);
        }
        // for i in 0..256 {
        //     println!(
        //         "i: {i}, cgram: b: {}, g: {}, r: {}",
        //         (self.cgram[i] >> 10) & 0x1F,
        //         (self.cgram[i] >> 5) & 0x1F,
        //         self.cgram[i] & 0x1F
        //     );
        // }
    }

    fn get_xy_color(&self, x: u16, y: u16) -> u16 {
        let bg_map_index = (y / 8) * 32 + x / 8;
        let bg_map_addr = (self.bg_screen_base_and_size[0].screen_base() as usize * 2048
            + bg_map_index as usize * 2)
            & 0xFFFE;
        // println!("bg_map_base: 0x{:x}", bg_map_addr);

        let bg_map_entry = BGMapEntry::from_bytes([
            self.vram[bg_map_addr as usize],
            self.vram[bg_map_addr as usize + 1],
        ]);
        let tile_addr =
            // (self.bg_character_base & 0x000F) * 4096 + bg_map_entry.character_number() as u16 * 16;
            (self.bg_character_base & 0x000F) * 4096 + bg_map_entry.character_number() as u16 * 16;
        // debug!("x: {x}, y: {y}, tile_addr: 0x{:x}", tile_addr);
        // debug!("character_number: {:x}", bg_map_entry.character_number());
        // debug!("pallet_number: {:x}", bg_map_entry.pallet_number());
        // debug!("bg_priority: {}", bg_map_entry.bg_priority());
        // debug!("flip_x: {}", bg_map_entry.flip_x());
        // debug!("flip_y: {}", bg_map_entry.flip_y());

        let tile_x = 7 - x % 8;
        let tile_y = y % 8;

        let mut tx = tile_x;
        let mut ty = tile_y;
        if bg_map_entry.flip_x() {
            tx = 7 - tx;
        }
        if bg_map_entry.flip_y() {
            ty = 7 - ty;
        }

        let bit_addr = tile_addr + ty * 2;
        let pallet_index = ((self.vram[bit_addr as usize + 1] >> tx) & 1) << 1
            | (self.vram[bit_addr as usize] >> tx) & 1;

        let pallet_addr = 8 * bg_map_entry.pallet_number() as u16 + pallet_index as u16;
        // let color = self.cgram[pallet_addr as usize + 1] << 8 | self.cgram[pallet_addr as usize];
        let color = self.cgram[pallet_addr as usize];
        color
    }
}

#[bitfield(bits = 16)]
struct BGMapEntry {
    character_number: B10,
    pallet_number: B3,
    bg_priority: bool,
    flip_x: bool,
    flip_y: bool,
}

#[bitfield(bits = 8)]
#[derive(Copy, Clone, Debug)]
struct BGScreenBaseSize {
    screen_size: B2,
    screen_base: B6,
}
