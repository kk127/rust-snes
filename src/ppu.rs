use crate::context;
use modular_bitfield::prelude::*;

use log::debug;
trait Context: context::Timing + context::Interrupt {}
impl<T: context::Timing + context::Interrupt> Context for T {}

pub struct Ppu {
    pub screen: [u16; 256 * 224],
    pub frame_number: u64,
    counter: u64,

    x: u16,
    y: u16,

    vram: [u8; 0x10000], // 64KB
    cgram: [u16; 0x100], // 512B
    oam: [u8; 0x220],    // 544B

    // Ppu control registers
    display_control: DisplayCtrl,           // $2100, $2133
    object_size_and_base: ObjectSizAndBase, // $2101
    main_screen_desination: u8,             // $212C
    sub_screen_destination: u8,             // $212D

    // BG control registers
    bg_mode_and_character_size: u8,                 // $2105
    mosaic_size_and_enable: MosaicSizeAndEnable,    // $2106
    bg_screen_base_and_size: [BGScreenBaseSize; 4], // $2107, $2108, $2109, $210A
    bg_tile_base_addr: [u8; 4],                     // $210B, $210C
    bg_hofs: [u16; 4],                              // $210D, $210F, $2111, $2113
    bg_vofs: [u16; 4],                              // $210E, $2110, $2112, $2114
    bg_old: u8,
    m7_hofs: u16,
    m7_vofs: u16,
    m7_old: u8,

    // Oam control registers
    oam_addr_and_priority_rotation: OamAddrAndPriorityRotation, // $2102, $2103
    oam_addr: u16,
    oam_lsb: u8, //

    // I/O port registers
    vram_mode: VramAddrIncMode, // $2115
    vram_addr: u16,             // $2116, $2117
    vram_prefetch: [u8; 2],
    palette_cgram_addr: u16, // $2121
    palette_cgram_lsb: u8,

    // Rotational and scaling registers
    rotation_scaling_setting: RotatinScalingSetting, // $211A
    rotation_scaling_param: RotationScalingParam,    // $211B, $211C, $211D, $211E, $211F, $2120

    // Window control registers
    window_position: [WindowPosition; 2], // $2126, $2127, $2128, $2129
    window_mask_settings: WindowMask,     // $2123, $2124, $2125
    window_mask_logic: WindowMaskLogic,   // $212A, $212B
    window_main_screen_disable: WindowAreaDisable, // $212E
    window_sub_screen_disable: WindowAreaDisable, // $212F

    // Color math registers
    color_math_ctrl: ColorMathCtrl, // $2130, $2131
    color_math_sub_screen_backdrop_color: ColorMathSubscreenBackdropColor, // $2132

    // Math Multiply and Devide registers
    mpy: i32, // $2134, $2135, $2136

    // Timers and Status
    h_counter_latch: u16, //$213C
    v_counter_latch: u16, //$213D
    hv_latched: bool,
    h_flipflopped: bool,
    v_flipflopped: bool,
    obj_time_overflow: bool,
    obj_range_overflow: bool,
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
            oam: [0; 0x220],
            display_control: Default::default(),
            object_size_and_base: Default::default(),
            main_screen_desination: 0,
            sub_screen_destination: 0,
            bg_mode_and_character_size: 0,
            mosaic_size_and_enable: Default::default(),
            bg_screen_base_and_size: [BGScreenBaseSize::new(); 4],
            bg_tile_base_addr: [0; 4],
            bg_hofs: [0; 4],
            bg_vofs: [0; 4],
            bg_old: 0,
            m7_hofs: 0,
            m7_vofs: 0,
            m7_old: 0,

            oam_addr_and_priority_rotation: Default::default(),
            oam_addr: 0,
            oam_lsb: 0,

            vram_mode: Default::default(),
            vram_addr: 0,
            vram_prefetch: [0; 2],
            palette_cgram_addr: 0,
            palette_cgram_lsb: 0,

            rotation_scaling_setting: Default::default(),
            rotation_scaling_param: Default::default(),

            window_position: Default::default(),
            window_mask_settings: Default::default(),
            window_mask_logic: Default::default(),
            window_main_screen_disable: Default::default(),
            window_sub_screen_disable: Default::default(),

            color_math_ctrl: Default::default(),
            color_math_sub_screen_backdrop_color: Default::default(),

            mpy: 1,

            h_counter_latch: 0,
            v_counter_latch: 0,
            hv_latched: false,
            h_flipflopped: false,
            v_flipflopped: false,
            obj_range_overflow: false,
            obj_time_overflow: false,
        }
    }
}

impl Ppu {
    pub fn read(&mut self, addr: u16, ctx: &mut impl Context) -> u8 {
        match addr {
            0x2134 => self.mpy as u8,
            0x2135 => (self.mpy >> 8) as u8,
            0x2136 => (self.mpy >> 16) as u8,
            0x2137 => {
                // TODO Three situations that load H/V counter values into the latch
                //  Doing a dummy-read from SLHV (Port 2137h) by software
                //  Switching WRIO (Port 4201h) Bit7 from 1-to-0 by software
                //  Lightgun High-to-Low transition (Pin6 of 2nd Controller connector)

                self.h_counter_latch = self.x;
                self.v_counter_latch = self.y;
                self.hv_latched = true;
                // TODO Return ppu open bus?
                0
            }
            0x2138 => {
                let ret = if self.oam_addr < 0x200 {
                    self.oam[self.oam_addr as usize]
                } else {
                    self.oam[(self.oam_addr & 0x21F) as usize]
                };
                self.oam_addr = (self.oam_addr + 1) & 0x3FF;
                // TODO Check whether to use the open bus value due to reading a value less than 8 bits.
                ret
            }
            0x2139 | 0x213A => {
                let index = (addr - 0x2139) as usize;
                let ret = self.vram_prefetch[index];
                if self.vram_mode.is_incremet_after_high_bit() == (index == 1) {
                    let vram_addr = self.vram_mode.get_transration(self.vram_addr) as usize * 2;
                    self.vram_prefetch[0] = self.vram[vram_addr];
                    self.vram_prefetch[1] = self.vram[vram_addr + 1];
                    self.vram_addr = (self.vram_addr + self.vram_mode.get_inc()) & 0x7FFF;
                }
                ret
            }
            0x213B => {
                let cgram_data = self.cgram[self.palette_cgram_addr as usize / 2];
                let ret = if self.palette_cgram_addr & 1 == 0 {
                    cgram_data as u8
                } else {
                    // TODO 2nd Access: Upper 7 bits (odd address) (upper 1bit = PPU2 open bus)
                    (cgram_data >> 8) as u8
                };
                self.palette_cgram_addr = (self.palette_cgram_addr + 1) & 0x1FF;
                ret
            }
            0x213C => {
                self.h_flipflopped = !self.h_flipflopped;
                if self.h_flipflopped {
                    self.h_counter_latch as u8
                } else {
                    // TODO Check whether to use the open bus value due to reading a value less than 8 bits.
                    (self.h_counter_latch >> 8) as u8 & 1
                }
            }
            0x213D => {
                self.v_flipflopped = !self.v_flipflopped;
                if self.v_flipflopped {
                    self.v_counter_latch as u8
                } else {
                    // TODO Check whether to use the open bus value due to reading a value less than 8 bits.
                    (self.v_counter_latch >> 8) as u8 & 1
                }
            }
            0x213E => {
                // bit0..=3 ppu1 5C77 version number = 1
                // bit5     Always read back as main processor (0: Main, 1: Helper)
                let mut ret = 1;

                ret |= (self.obj_range_overflow as u8) << 6;
                ret |= (self.obj_time_overflow as u8) << 7;
                // TODO Check whether to use the open bus value due to reading a value less than 8 bits.
                ret
            }
            0x213F => {
                // Ppu version = 1;
                // Frame rate = 0 (60Hz)
                let mut ret = 1;

                ret |= (self.hv_latched as u8) << 6;
                ret |= (self.frame_number as u8 & 1) << 7;

                self.hv_latched = false;
                // TODO resets the two OPHCT/OPVCT 1st/2nd-read flipflops.

                // TODO check ppu open bus
                ret
            }
            _ => {
                unimplemented!("Read unimplemeted, addr: {:x}", addr);
            }
        }
    }

    pub fn write(&mut self, addr: u16, data: u8, ctx: &mut impl Context) {
        debug!("PPU write, addr: {:x}, data: {:x}", addr, data);
        match addr {
            0x2100 => self.display_control.bytes[0] = data,
            0x2101 => self.object_size_and_base.bytes[0] = data,
            0x2102 | 0x2103 => {
                let index = (addr - 0x2102) as usize;
                self.oam_addr_and_priority_rotation.bytes[index] = data;
                self.oam_addr = self.oam_addr_and_priority_rotation.addr() << 1;
            }
            0x2104 => {
                if self.oam_addr < 0x200 {
                    if self.oam_addr & 1 == 0 {
                        self.oam_lsb = data;
                    } else {
                        self.oam[self.oam_addr as usize - 1] = self.oam_lsb;
                        self.oam[self.oam_addr as usize] = data;
                    }
                } else {
                    self.oam[(self.oam_addr & 0x21F) as usize] = data;
                }
                self.oam_addr = (self.oam_addr + 1) & 0x3FF;
            }
            0x2105 => self.bg_mode_and_character_size = data,
            0x2106 => self.mosaic_size_and_enable.bytes[0] = data,
            0x2107..=0x210A => {
                let index = (addr - 0x2107) as usize;
                self.bg_screen_base_and_size[index].bytes[0] = data;
            }

            0x210B | 0x210C => {
                let index = (addr - 0x210B) as usize * 2;
                self.bg_tile_base_addr[index] = data & 0x0F;
                self.bg_tile_base_addr[index + 1] = data >> 4;
            }
            0x210D | 0x210F | 0x2111 | 0x2113 => {
                let index = (addr - 0x210D) as usize / 2;
                self.bg_hofs[index] =
                    (data as u16) << 8 | (self.bg_old & !7) as u16 | (self.bg_hofs[index] >> 8) & 7;
                self.bg_old = data;

                if index == 0 {
                    self.m7_hofs = (data as u16) << 8 | self.m7_old as u16;
                    self.m7_old = data;
                }
            }

            0x210E | 0x2110 | 0x2112 | 0x2114 => {
                let index = (addr - 0x210E) as usize / 2;
                self.bg_vofs[index] = (data as u16) << 8 | self.bg_old as u16;
                self.bg_old = data;

                if index == 0 {
                    self.m7_vofs = (data as u16) << 8 | self.m7_old as u16;
                    self.m7_old = data;
                }
            }

            0x2115 => self.vram_mode.bytes[0] = data,
            0x2116 => {
                self.vram_addr = self.vram_addr & 0x7F00 | data as u16;
                self.vram_prefetch[0] = self.vram[self.vram_addr as usize * 2];
                self.vram_prefetch[1] = self.vram[self.vram_addr as usize * 2 + 1];
            }
            0x2117 => {
                self.vram_addr = self.vram_addr & 0x00FF | ((data & 0x7F) as u16) << 8;
                self.vram_prefetch[0] = self.vram[self.vram_addr as usize * 2];
                self.vram_prefetch[1] = self.vram[self.vram_addr as usize * 2 + 1];
            }
            0x2118 | 0x2119 => {
                let offset = addr - 0x2118;
                let vram_addr = self.vram_mode.get_transration(self.vram_addr) * 2 + offset;
                // debug!("vram_addr: 0x{:x}, data: 0x{:x}", vram_addr, data);
                debug!(
                    "VRAM: {:04X} = {data:02X}, addr: {:04X}",
                    self.vram_addr, vram_addr
                );
                self.vram[vram_addr as usize] = data;
                if self.vram_mode.is_incremet_after_high_bit() == (offset == 1) {
                    self.vram_addr = (self.vram_addr + self.vram_mode.get_inc()) & 0x7FFF;
                }
            }
            0x211A => self.rotation_scaling_setting.bytes[0] = data,
            0x211B => {
                self.rotation_scaling_param.a = (data as u16) << 8 | self.m7_old as u16;
                self.m7_old = data;
                self.mpy = (self.rotation_scaling_param.a as i16 as i32)
                    * (self.rotation_scaling_param.b as i8 as i32);
            }
            0x211C => {
                self.rotation_scaling_param.b = (data as u16) << 8 | self.m7_old as u16;
                self.m7_old = data;
                self.mpy = (self.rotation_scaling_param.a as i16 as i32)
                    * (self.rotation_scaling_param.b as i8 as i32);
            }
            0x211D => {
                self.rotation_scaling_param.c = (data as u16) << 8 | self.m7_old as u16;
                self.m7_old = data;
            }
            0x211E => {
                self.rotation_scaling_param.d = (data as u16) << 8 | self.m7_old as u16;
                self.m7_old = data;
            }
            0x211F => {
                self.rotation_scaling_param.x = (data as u16) << 8 | self.m7_old as u16;
                self.m7_old = data;
            }
            0x2120 => {
                self.rotation_scaling_param.y = (data as u16) << 8 | self.m7_old as u16;
                self.m7_old = data;
            }
            0x2121 => self.palette_cgram_addr = data as u16 * 2,
            0x2122 => {
                if self.palette_cgram_addr & 1 == 0 {
                    self.palette_cgram_lsb = data;
                } else {
                    self.cgram[self.palette_cgram_addr as usize / 2] =
                        (data as u16) << 8 | self.palette_cgram_lsb as u16;
                }
                self.palette_cgram_addr = (self.palette_cgram_addr + 1) & 0x1FF;
            }
            0x2123 => {
                self.window_mask_settings.bg[0].bytes[0] = data & 0x0F;
                self.window_mask_settings.bg[1].bytes[0] = data >> 4;
            }
            0x2124 => {
                self.window_mask_settings.bg[2].bytes[0] = data & 0x0F;
                self.window_mask_settings.bg[3].bytes[0] = data >> 4;
            }
            0x2125 => {
                self.window_mask_settings.obj.bytes[0] = data & 0x0F;
                self.window_mask_settings.math.bytes[0] = data >> 4;
            }
            0x2126 => self.window_position[0].left = data,
            0x2127 => self.window_position[0].right = data,
            0x2128 => self.window_position[1].left = data,
            0x2129 => self.window_position[1].right = data,
            0x212A => self.window_mask_logic.bytes[0] = data,
            0x212B => self.window_mask_logic.bytes[1] = data,
            0x212C => self.main_screen_desination = data,
            0x212D => self.sub_screen_destination = data,
            0x212E => self.window_main_screen_disable.bytes[0] = data,
            0x212F => self.window_sub_screen_disable.bytes[0] = data,

            0x2130 => self.color_math_ctrl.bytes[0] = data,
            0x2131 => self.color_math_ctrl.bytes[1] = data,
            // TODO additional process
            0x2132 => self.color_math_sub_screen_backdrop_color.bytes[0] = data,
            0x2133 => self.display_control.bytes[1] = data,
            0x2134..=0x213F => {
                unreachable!("Write PPU register, addr: {:x}, data: {:x}", addr, data);
            }

            _ => {
                // println!("Write unimplemeted, addr: {:x}, data: {:x}", addr, data);
                debug!("Write unimplemeted, addr: {:x}, data: {:x}", addr, data);
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

                    ctx.set_nmi_flag(false);

                    self.frame_number += 1;
                    debug!("frame_number: {}", self.frame_number);
                    debug!("cgaram: {:?}", self.cgram);
                    debug!("vram: {:?}", self.vram);
                    // 0xf800..0xf800 + 32 * 32
                    for i in 0..32 {
                        for j in 0..32 {
                            let addr = 0xf800 + i * 32 + j * 2;
                            debug!(
                                "vram[0x{:x}]: 0x{:x}",
                                addr,
                                (self.vram[addr as usize + 1] as u16) << 8
                                    | self.vram[addr as usize] as u16
                            );
                        }
                    }
                }
            }

            if self.x == 0 && self.y == 225 {
                ctx.set_nmi_flag(true);
            }

            if self.x == 10 && self.y == 225 {
                if !self.display_control.force_blank() {
                    self.oam_addr = self.oam_addr_and_priority_rotation.addr() << 1;
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
        // debug!("x: {}, y: {}, bg_map_base: 0x{:x}", x, y, bg_map_addr);
        // println!("bg_map_base: 0x{:x}", bg_map_addr);

        let bg_map_entry = BGMapEntry::from_bytes([
            self.vram[bg_map_addr as usize],
            self.vram[bg_map_addr as usize + 1],
        ]);
        // debug!("bg_map_entry: {:?}", bg_map_entry);
        let tile_addr = (self.bg_tile_base_addr[0] as u16 & 0x000F) * 4096
            + bg_map_entry.character_number() as u16 * 16;
        // (self.bg_character_base & 0x000F) * 8 * 1024 + bg_map_entry.character_number() as u16 * 16;
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
#[derive(Default)]
struct DisplayCtrl {
    brightness: B4,
    #[skip]
    __: B3,
    force_blank: bool,
    v_scanning: bool,
    obj_v_direction_display: bool,
    bg_v_direction_display: bool,
    horizontal_pseudo_512mode: bool,
    #[skip]
    __: B2,
    extbg_mode: bool,
    external_sync: bool,
}

#[bitfield(bits = 16)]
#[derive(Debug)]
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

#[bitfield(bits = 8)]
#[derive(Default)]
struct ObjectSizAndBase {
    base_addr_for_obj_tiles: B3,
    gap_between_obj: B2,
    obj_size_selection: ObjectSizeSelection,
}

#[bits = 3]
#[derive(BitfieldSpecifier, Debug, Copy, Clone)]
enum ObjectSizeSelection {
    Size8x8_16x16 = 0,
    Size8x8_32x32 = 1,
    Size8x8_64x64 = 2,
    Size16x16_32x32 = 3,
    Size16x16_64x64 = 4,
    Size32x32_64x64 = 5,
    Size16x32_32x64 = 6, // Undocumented
    Size16x32_32x32 = 7, // Undocumented
}

#[bitfield(bits = 16)]
#[derive(Default)]
struct OamAddrAndPriorityRotation {
    addr: B9,
    __: B6,
    priority_rotation: bool,
}

#[bitfield(bits = 8)]
#[derive(Default)]
struct RotatinScalingSetting {
    h_flip: bool,
    v_flip: bool,
    __: B4,
    screen_over: B2,
}

#[derive(Default)]
struct RotationScalingParam {
    a: u16,
    b: u16,
    c: u16,
    d: u16,
    x: u16,
    y: u16,
}

#[derive(Default)]
struct WindowPosition {
    left: u8,
    right: u8,
}

#[derive(Default)]
struct WindowMask {
    bg: [MaskSettings; 4],
    obj: MaskSettings,
    math: MaskSettings,
}

#[bitfield(bits = 8)]
#[derive(BitfieldSpecifier, Default)]
struct MaskSettings {
    window1: MaskSetting,
    window2: MaskSetting,
    __: B4,
}

#[bitfield(bits = 2)]
#[derive(BitfieldSpecifier)]
struct MaskSetting {
    enable: bool,
    outside: bool,
}

#[bitfield(bits = 16)]
#[derive(Default)]
struct WindowMaskLogic {
    bg1: MaskLogic,
    bg2: MaskLogic,
    bg3: MaskLogic,
    bg4: MaskLogic,
    obj: MaskLogic,
    math: MaskLogic,
    __: B4,
}

#[derive(BitfieldSpecifier)]
#[bits = 2]
#[derive(Default)]
enum MaskLogic {
    #[default]
    Or = 0,
    And = 1,
    Xor = 2,
    Xnor = 3,
}

#[bitfield(bits = 8)]
#[derive(Default)]
struct WindowAreaDisable {
    bg1: bool,
    bg2: bool,
    bg3: bool,
    bg4: bool,
    obj: bool,
    __: B3,
}

#[bitfield(bits = 8)]
#[derive(Default)]
struct MosaicSizeAndEnable {
    enable: B4,
    size: B4,
}

#[bitfield(bits = 16)]
#[derive(Default)]
struct ColorMathCtrl {
    direct_color: bool,
    sub_screen_enable: bool,
    #[skip]
    __: B2,
    enable: ColorMathEnable,
    force_main_screen_black: ForceMainScreenBlack,
    kind: B6,
    half_color: bool,
    subtract: bool,
}

#[bits = 2]
#[derive(BitfieldSpecifier, Default)]
enum ColorMathEnable {
    #[default]
    Always = 0,
    MathWindow = 1,
    NotMathWin = 2,
    Never = 3,
}

#[bits = 2]
#[derive(BitfieldSpecifier, Default)]
enum ForceMainScreenBlack {
    #[default]
    Never = 0,
    MathWindow = 1,
    NotMathWin = 2,
    Always = 3,
}

#[bitfield(bits = 8)]
#[derive(Default)]
struct ColorMathSubscreenBackdropColor {
    intensity: B5,
    r: bool,
    g: bool,
    b: bool,
}
