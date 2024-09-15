use crate::context;
use modular_bitfield::prelude::*;

use log::{debug, warn};
trait Context: context::Timing + context::Interrupt  {}
impl<T: context::Timing + context::Interrupt> Context for T {}

const FRAME_HEIGHT: usize = 224;
const FRAME_WIDTH: usize = 256;

const BG_MODE_BPP: [&[usize]; 8] = [
    &[2, 2, 2, 2],  // Mode0
    &[4, 4, 2],     // Mode1
    &[4, 4],        // Mode2
    &[8, 4],        // Mode3
    &[8, 2],        // Mode4
    &[4, 2],        // Mode5
    &[4],           // Mode6
    // TODO EXTBG
    &[8],           // Mode7 
];

const OBJ_PRIORITY: [u8; 4] = [10, 7, 4, 1];

pub struct Ppu {
    pub frame: [u16; FRAME_WIDTH * FRAME_HEIGHT],
    pub frame_number: u64,
    counter: u64,
    main_screen: [PixelInfo; FRAME_WIDTH],
    sub_screen: [PixelInfo; FRAME_WIDTH],

    x: u16,
    y: u16,

    is_hblank: bool,
    is_vblank: bool,
    is_hdma_reload: bool,
    is_hdma_transfer: bool,

    pub vram: [u8; 0x10000], // 64KB
    cgram: [u16; 0x100], // 512B
    pub oam: [u8; 0x220],    // 544B

    // Ppu control registers
    display_control: DisplayCtrl,               // $2100, $2133
    object_size_and_base: ObjectSizeAndBase,     // $2101
    screen_main_designation: ScreenDesignation, // $212C
    screen_sub_designation: ScreenDesignation,  // $212D

    // BG control registers
    bg_ctrl: BgCtrl,                                // $2105
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
    window_main_designation: ScreenDesignation, // $212E
    window_sub_designation: ScreenDesignation, // $212F

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

    auto_joypad_read: bool,
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
            frame: [0; 256 * 224],
            frame_number: 0,
            counter: 0,
            main_screen: [Default::default(); FRAME_WIDTH],
            sub_screen: [Default::default(); FRAME_WIDTH],

            x: 0,
            y: 0,

            is_hblank: false,
            is_vblank: false,
            is_hdma_reload: false,
            is_hdma_transfer: false,
        
            vram: [0; 0x10000],
            cgram: [0; 0x100],
            oam: [0; 0x220],
            display_control: Default::default(),
            object_size_and_base: Default::default(),
            screen_main_designation: Default::default(),
            screen_sub_designation: Default::default(),
            bg_ctrl: Default::default(),
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
            window_main_designation: Default::default(),
            window_sub_designation: Default::default(),

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

            auto_joypad_read: false,
        }
    }
}

impl Ppu {
    pub(crate) fn read(&mut self, addr: u16, ctx: &mut impl Context) -> u8 {
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
                warn!("Read Ppu write only register, addr: {:x}", addr);
                // TODO return ppu open bus
                0
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
            0x2105 => self.bg_ctrl.bytes[0] = data,
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
            0x212C => self.screen_main_designation.bytes[0] = data,
            0x212D => self.screen_sub_designation.bytes[0] = data,
            0x212E => self.window_main_designation.bytes[0] = data,
            0x212F => self.window_sub_designation.bytes[0] = data,

            0x2130 => self.color_math_ctrl.bytes[0] = data,
            0x2131 => self.color_math_ctrl.bytes[1] = data,
            0x2132 => {
                let intensity = data & 0x1F;
                if data >> 5 & 1 == 1 {
                    self.color_math_sub_screen_backdrop_color.r = intensity;
                }
                if data >> 6 & 1 == 1 {
                    self.color_math_sub_screen_backdrop_color.g = intensity;
                }
                if data >> 7 & 1 == 1 {
                    self.color_math_sub_screen_backdrop_color.b = intensity;
                }
            }
            0x2133 => self.display_control.bytes[1] = data,
            0x2134..=0x213F => {
                warn!("Write PPU read only register, addr: {:x}, data: {:x}", addr, data);
            }

            _ => {
                debug!("Write unimplemeted, addr: {:x}, data: {:x}", addr, data);
            }
        }
    }

    pub fn tick(&mut self, ctx: &mut impl Context) {
        loop {
            if self.counter + 4 > ctx.now() {
                break;
            }

            self.counter += 4;

            self.x += 1;
            if self.x == 340 {
                self.x = 0;
                self.y += 1;


                if self.y == 262 {
                    self.y = 0;

                    self.is_vblank = false;
                    ctx.set_nmi_flag(false);

                    self.frame_number += 1;
                    debug!("frame_number: {}", self.frame_number);
                    debug!("cgaram: {:?}", self.cgram);
                    debug!("vram: {:?}", self.vram);
                }

                if self.y == 225 {
                    debug!("VBlank start");
                    self.is_vblank = true;
                }
            }

            if self.x == 0 && self.y == 225 {
                ctx.set_nmi_flag(true);
            }

            if self.x == 1 {
                self.is_hblank = false;
            }

            if (self.x, self.y) == (6, 0) {
                self.is_hdma_reload = true;
            }

            if self.x == 10 && self.y == 225 {
                if !self.display_control.force_blank() {
                    self.oam_addr = self.oam_addr_and_priority_rotation.addr() << 1;
                }
            }

            if (self.x, self.y) == (33, 225) {
                self.auto_joypad_read = true;
            }

            if self.x == 134 {
                // DRAM refresh
                ctx.elapse(40);
            }
            if self.x == 278 && (0..=224).contains(&self.y) {
                self.is_hdma_transfer = true;
            }

            if self.x == 274 {
                self.is_hblank = true;
            }

            if self.x == 10 && self.y == 225 {
                if !self.display_control.force_blank() {
                    self.oam_addr = self.oam_addr_and_priority_rotation.addr() << 1;
                }
            }

            if self.x == 22 && (1..225).contains(&self.y) {
                self.render_line(self.y - 1);
            }

            match ctx.get_hv_irq_enable() {
                1 => {
                    if self.x == ctx.get_h_count() {
                        ctx.set_irq(true);
                    }
                }
                2 => {
                    if self.x == 0 && self.y == ctx.get_v_count() {
                        ctx.set_irq(true);
                    }
                }
                3 => {
                    if self.x == ctx.get_h_count() && self.y == ctx.get_v_count() {
                        ctx.set_irq(true);
                    }
                }
                _ => {}
            }
        }

        let mut counter = ctx.counter_mut();
        counter.frame = self.frame_number;
        counter.x = self.x as u64;
        counter.y = self.y as u64;
    }

    fn render_line(&mut self, y: u16) {
        self.render_bg(y);
        self.render_obj(y);
        self.color_math(y);
    }

    fn render_bg(&mut self, y: u16) {
        let bg_mode = self.bg_ctrl.bg_mode();
        let bpp_mode = BG_MODE_BPP[bg_mode as usize];
        for i in 0..FRAME_WIDTH {
            self.main_screen[i] = PixelInfo::new(self.cgram[0], 13, Layer::Backdrop);
            self.sub_screen[i] = PixelInfo::new(self.color_math_sub_screen_backdrop_color.get_bgr(), 13, Layer::Backdrop);
        }
        for (bg_index, &bpp) in bpp_mode.iter().enumerate() {
            let (tile_w_num, tile_h_num) = self.bg_screen_base_and_size[bg_index].get_tile_num();
            let tile_size = self.bg_ctrl.get_tile_size(bg_index);
            let tile_base_addr = self.bg_tile_base_addr[bg_index] as usize * 8 * 1024;
            debug!("tile base addr: 0x{:x}", tile_base_addr);
            let screen_width = tile_w_num * tile_size;
            let screen_height = tile_h_num * tile_size;


            for x in 0..FRAME_WIDTH {
                let screen_x = (x + self.bg_hofs[bg_index] as usize) % screen_width;
                let screen_y = (y as usize + self.bg_vofs[bg_index] as usize) % screen_height;

                let mut bg_map_base_addr = self.bg_screen_base_and_size[bg_index].get_bg_map_base_addr();
                if x == 0 {
                    debug!("bg_map_base_addr: 0x{:x}", bg_map_base_addr);
                }
                let mut tile_x_index = screen_x / tile_size;
                let mut tile_y_index = screen_y / tile_size;
                if tile_x_index >= 32 {
                    tile_x_index %= 32;
                    bg_map_base_addr += 2 * 1024;
                }
                if tile_y_index >= 32 {
                    tile_y_index %= 32;
                    bg_map_base_addr += 2 * 2 * 1024;
                }

                let map_entry_addr = (bg_map_base_addr + 2 * (tile_y_index * 32 + tile_x_index)) & 0xFFFE;
                let map_entry = BGMapEntry::from_bytes([
                    self.vram[map_entry_addr],
                    self.vram[map_entry_addr + 1],
                ]);

                let mut tile_index = map_entry.character_number() as usize;
                let mut pixel_x = (screen_x % tile_size) ^ if map_entry.flip_x() { tile_size -1 } else { 0 };
                let mut pixel_y = (screen_y % tile_size) ^ if map_entry.flip_y() { tile_size -1 } else { 0 };
                if pixel_x >= 8 {
                    tile_index += 0x01;
                    pixel_x %= 8;
                }
                if pixel_y >= 8 {
                    tile_index += 0x10;
                    pixel_y %= 8;
                }

                let tile_addr  = tile_base_addr + tile_index * bpp  * 8;
                let mut color_index = 0;
                for i in 0..bpp/2 {
                    let bit_addr = (tile_addr + i * 16 + pixel_y * 2) & 0xFFFE;
                    let low = (self.vram[bit_addr] >> (7 - pixel_x)) & 1;
                    let high = (self.vram[bit_addr + 1] >> (7 - pixel_x)) & 1;
                    color_index |= low << (i * 2);
                    color_index |= high << (i * 2 + 1);
                } 

                let is_high = map_entry.bg_priority();
                if color_index != 0 {
                    let cgram_base_addr = if self.bg_ctrl.bg_mode() == 0 {
                        bg_index * 0x20
                    } else {
                        0
                    };
                    let cgram_addr = (cgram_base_addr + map_entry.pallet_number() as usize * (1 << bpp) + color_index as usize) & 0xFF;
                    let color = self.cgram[cgram_addr];
                    if self.screen_main_designation.get_bg_enable(bg_index) {
                        let priority = self.get_bg_layer_priority(bg_index as u8, is_high);
                        if priority < self.main_screen[x].priority {
                            self.main_screen[x] = PixelInfo::new(color, priority, Layer::BG(bg_index as u8));
                        }
                    }
                    if self.screen_sub_designation.get_bg_enable(bg_index) {
                        let priority = self.get_bg_layer_priority(bg_index as u8, is_high);
                        if priority < self.sub_screen[x].priority {
                            self.sub_screen[x] = PixelInfo::new(color, priority, Layer::BG(bg_index as u8));
                        }
                    }
                    // self.frame[y as usize * FRAME_WIDTH + x] = color;
                }
            }
        }
    }

    fn render_obj(&mut self, y: u16) {
        for i in 0..128 {
            let oam_entry = OamEntry::from_bytes(self.oam[i * 4..i * 4 + 4].try_into().unwrap());
            let addition_addr = 0x200 + (i / 4) ;
            let addition_offset = i % 4;
            let upper_x = ((self.oam[addition_addr] >> (addition_offset * 2)) & 1) as usize;
            let obj_size_index = ((self.oam[addition_addr] >> (addition_offset * 2 + 1)) & 1) as usize;

            let obj_pos_x = (upper_x << 8) | oam_entry.x() as usize;
            let obj_pos_y =  oam_entry.y() as usize;

            let obj_size = self.object_size_and_base.obj_size()[obj_size_index];

            for offset_y in 0..obj_size {
                let pixel_y = (obj_pos_y + offset_y) % 256;
                if pixel_y != y as usize {
                    continue;
                }
                for offset_x in 0..obj_size {
                    let pixel_x = (obj_pos_x + offset_x) % 512;
                    if pixel_x >= 256 {
                        continue;
                    }

                    let mut tile_x = if oam_entry.attribute().x_flip() { (obj_size -1) ^ offset_x } else { offset_x };
                    let mut tile_y = if oam_entry.attribute().y_flip() { (obj_size -1) ^ offset_y } else { offset_y };

                    let mut tile_index = ((oam_entry.attribute().tile_page() as usize) << 8) |  oam_entry.tile_number() as usize;
                    // x方向は0x01ずれる
                    tile_index = (tile_index & 0x1F0) | (((tile_index & 0x0F) + tile_x / 8 ) & 0x0F);
                    // y方向は0x10ずれる
                    tile_index = (((tile_index & 0x1F0) + tile_y / 8 * 0x10) & 0x1F0) | (tile_index & 0x0F);

                    tile_x %= 8;
                    tile_y %= 8;

                    let mut tile_base_addr = self.object_size_and_base.base_addr_for_obj_tiles() as usize * 16 * 1024;
                    if oam_entry.attribute().tile_page() == 1 {
                        tile_base_addr += self.object_size_and_base.gap_between_obj() as usize * 8 * 1024;
                    }
                    tile_base_addr &= 0xFFFF;


                    let tile_addr = tile_base_addr + tile_index * 32;
                    let mut color_index = 0;
                    for i in 0..2 {
                        let bit_addr = (tile_addr + i * 16 + tile_y * 2) & 0xFFFE;
                        let low = (self.vram[bit_addr] >> (7 - tile_x)) & 1;
                        let high = (self.vram[bit_addr + 1] >> (7 - tile_x)) & 1;
                        color_index |= low << (i * 2);
                        color_index |= high << (i * 2 + 1);
                    } 
                    
                    if color_index == 0 {
                        continue;
                    }
                    let obj_priority = OBJ_PRIORITY[oam_entry.attribute().priority() as usize];
                    if obj_priority < self.main_screen[pixel_x].priority {
                        let cgram_addr =  128 + oam_entry.attribute().palette_number() as usize * 16 + color_index as usize;
                        let color = self.cgram[cgram_addr];
                        let layer = if (0..=3).contains(&oam_entry.attribute().palette_number()) {
                            Layer::ObjPallete0_3
                        } else {
                            Layer::ObjPallete4_7
                        };
                        self.main_screen[pixel_x] = PixelInfo::new(color, obj_priority, layer);
                    } 
                    if obj_priority < self.sub_screen[pixel_x].priority {
                        let cgram_addr =  128 + oam_entry.attribute().palette_number() as usize * 16 + color_index as usize;
                        let color = self.cgram[cgram_addr];
                        let layer = if (0..=3).contains(&oam_entry.attribute().palette_number()) {
                            Layer::ObjPallete0_3
                        } else {
                            Layer::ObjPallete4_7
                        };
                        self.sub_screen[pixel_x] = PixelInfo::new(color, obj_priority, layer);
                    }

                }
            }
        }
    }

    fn color_math(&mut self, y: u16) {
        let bright_ness = self.display_control.brightness();
        for i in 0..FRAME_WIDTH {
            let mut main_color = self.main_screen[i];
            let mut sub_color = self.sub_screen[i];

            if bright_ness == 0 {
                main_color.r = 0;
                main_color.g = 0;
                main_color.b = 0;
                sub_color.r = 0;
                sub_color.g = 0;
                sub_color.b = 0;
            } else {
                main_color.r = ((main_color.r as u16 * (bright_ness + 1) as u16) / 16) as u8;
                main_color.g = ((main_color.g as u16 * (bright_ness + 1) as u16) / 16) as u8;
                main_color.b = ((main_color.b as u16 * (bright_ness + 1) as u16) / 16) as u8;
                sub_color.r = ((sub_color.r as u16 * (bright_ness + 1) as u16) / 16) as u8;
                sub_color.g = ((sub_color.g as u16 * (bright_ness + 1) as u16) / 16) as u8;
                sub_color.b = ((sub_color.b as u16 * (bright_ness + 1) as u16) / 16) as u8;
            }
            // let color = self.color_math_ctrl.calc_color(main_color, sub_color);

            if (self.color_math_ctrl.kind() >> (main_color.layer as u8)) & 1 == 1 {
                let mut color_r = 0;
                let mut color_g = 0;
                let mut color_b = 0;
                // main_color = self.color_math_ctrl.calc_color(main_color, sub_color);
                if self.color_math_ctrl.subtract() {
                    color_r = main_color.r.saturating_sub(sub_color.r);
                    color_g = main_color.g.saturating_sub(sub_color.g);
                    color_b = main_color.b.saturating_sub(sub_color.b);
                } else {
                    color_r = main_color.r + sub_color.r;
                    color_g = main_color.g + sub_color.g;
                    color_b = main_color.b + sub_color.b;
                }
                if self.color_math_ctrl.half_color() {
                    color_r >>= 1;
                    color_g >>= 1;
                    color_b >>= 1;
                }
                color_r = color_r.min(31);
                color_g = color_g.min(31);
                color_b = color_b.min(31);
                self.frame[y as usize * FRAME_WIDTH + i] = (color_b as u16) << 10 | (color_g as u16) << 5 | color_r as u16;
            } else {
                self.frame[y as usize * FRAME_WIDTH + i] = (main_color.b as u16) << 10 | (main_color.g as u16) << 5 | main_color.r as u16;
            }

        }
    }


    fn get_xy_color(&self, x: u16, y: u16) -> u16 {
        let bg_map_index = (y / 8) * 32 + x / 8;
        let bg_map_addr = (self.bg_screen_base_and_size[0].screen_base() as usize * 2048
            + bg_map_index as usize * 2)
            & 0xFFFE;
        // debug!("x: {}, y: {}, bg_map_base: 0x{:x}", x, y, bg_map_addr);

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

    #[rustfmt::skip]
    fn get_bg_layer_priority(&self, layer: u8, is_high: bool) -> u8 {
        match self.bg_ctrl.bg_mode() {
            0 => match layer {
                0 => if is_high { 2 } else {  5 },  // BG1
                1 => if is_high { 3 } else {  6 },  // BG2
                2 => if is_high { 8 } else { 11 },  // BG3
                3 => if is_high { 9 } else { 12 },  // BG4
                _ => unreachable!(),
            },
            1 => match layer {
                0 => if is_high { 2 } else { 5 },  // BG1
                1 => if is_high { 3 } else { 6 },  // BG2
                2 => match (self.bg_ctrl.is_bg3_priority_high(), is_high) {
                    (true, true)   =>  0,           // BG3.1a
                    (true, false)  => 11,           // BG3.0a
                    (false, true)  =>  8,           // BG3.1b
                    (false, false) => 12,           // BG3.0b
                },
                _ => unreachable!(),
            },
            2..=5 => match layer {
                0 => if is_high { 2 } else {  8 },  // BG1
                1 => if is_high { 5 } else { 11 },  // BG2
                _ => unreachable!(),
            },
            6 => match layer {
                0 => if is_high { 2 } else { 8 },  // BG1
                _ => unreachable!(),
            },
            7 => match layer {
                0 => 7,
                // TODO EXTBG
                1 => 11,
                _ => unreachable!(),
            }
            _ => unreachable!(),
        }    
    }

    pub fn is_auto_joypad_read(&mut self) -> bool {
        let ret = self.auto_joypad_read;
        self.auto_joypad_read = false;
        ret
    }
}

impl Ppu {
    pub fn is_hblank(&self) -> bool {
        self.is_hblank
    }
    pub fn is_vblank(&self) -> bool {
        self.is_vblank
    }

    pub fn is_hdma_reload_triggered(&mut self) -> bool {
        let ret = self.is_hdma_reload;
        self.is_hdma_reload = false;
        ret
    }

    pub fn is_hdma_transfer_triggered(&mut self) -> bool {
        let ret = self.is_hdma_transfer;
        self.is_hdma_transfer = false;
        ret
    }
}


#[derive(Default, Clone, Copy)]
struct PixelInfo {
    r: u8,
    g: u8,
    b: u8,
    priority: u8,
    layer: Layer,
}

impl PixelInfo {
    fn new(color: u16, priority: u8, layer: Layer) -> Self {
        let r = (color & 0x1F) as u8;
        let g = ((color >> 5) & 0x1F) as u8;
        let b = ((color >> 10) & 0x1F) as u8;
        PixelInfo { r, g, b, priority, layer }
    }
}

#[derive(Default, Clone, Copy)]
enum Layer {
    Bg1 = 0,
    Bg2 = 1,
    Bg3 = 2,
    Bg4 = 3,
    ObjPallete0_3 = 7, // (Always=Off)
    ObjPallete4_7 = 4,
    #[default]
    Backdrop = 5,
}

impl Layer {
    fn BG(bg_index: u8) -> Self {
        match bg_index {
            0 => Layer::Bg1,
            1 => Layer::Bg2,
            2 => Layer::Bg3,
            3 => Layer::Bg4,
            _ => unreachable!(),
        }
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

#[bitfield(bits = 8)]
#[derive(Default)]
struct ScreenDesignation {
    bg1_enable: bool,
    bg2_enable: bool,
    bg3_enable: bool,
    bg4_enable: bool,
    obj_enable: bool,
    #[skip]
    __: B3,
}

impl ScreenDesignation {
    fn get_bg_enable(&self, bg_index: usize) -> bool {
        match bg_index {
            0 => self.bg1_enable(),
            1 => self.bg2_enable(),
            2 => self.bg3_enable(),
            3 => self.bg4_enable(),
            _ => unreachable!(),
        }
    }
}

#[bitfield(bits = 8)]
#[derive(Default)]
struct BgCtrl {
    bg_mode: B3,
    is_bg3_priority_high: bool,
    tile_size: B4,
}

impl BgCtrl {
    fn get_tile_size(&self, bg_index: usize) -> usize {
        match (self.tile_size() >> bg_index) & 1 {
            0 => 8,
            1 => 16,
            _ => unreachable!(),
        }
    }
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

impl BGScreenBaseSize {
    fn get_tile_num(&self) -> (usize, usize) {
        match self.screen_size() {
            0 => (32, 32),
            1 => (64, 32),
            2 => (32, 64),
            3 => (64, 64),
            _ => unreachable!(),
        }
    }
}


impl BGScreenBaseSize {
    fn get_bg_map_base_addr(&self) -> usize {
        self.screen_base() as usize * 0x800
    }
}

#[bitfield(bits = 32)]
#[derive(Debug)]
struct OamEntry {
    x: B8,
    y: B8,
    tile_number: B8,
    attribute: Attribute
}

#[bitfield(bits = 8)]
#[derive(BitfieldSpecifier, Debug, Default)]
struct Attribute {
    tile_page: B1,
    palette_number: B3,
    priority: B2,
    x_flip: bool,
    y_flip: bool,
}


#[bitfield(bits = 8)]
#[derive(Default)]
struct ObjectSizeAndBase {
    base_addr_for_obj_tiles: B3,
    gap_between_obj: B2,
    obj_size_selection: ObjectSizeSelection,
}

impl ObjectSizeAndBase {
    fn obj_size(&self) -> [usize; 2] {
        match self.obj_size_selection() {
            ObjectSizeSelection::Size8x8_16x16 => [8, 16],
            ObjectSizeSelection::Size8x8_32x32 => [8, 32],
            ObjectSizeSelection::Size8x8_64x64 => [8, 64],
            ObjectSizeSelection::Size16x16_32x32 => [16, 32],
            ObjectSizeSelection::Size16x16_64x64 => [16, 64],
            ObjectSizeSelection::Size32x32_64x64 => [32, 64],
            ObjectSizeSelection::Size16x32_32x64 => [16, 32],
            ObjectSizeSelection::Size16x32_32x32 => [16, 32],
        }
    }
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

#[derive(Default)]
struct ColorMathSubscreenBackdropColor {
    r: u8,
    g: u8,
    b: u8,
}

impl ColorMathSubscreenBackdropColor {
    fn get_bgr(&self) -> u16 {
        (self.b as u16) << 10 | (self.g as u16) << 5 | self.r as u16
    }
}