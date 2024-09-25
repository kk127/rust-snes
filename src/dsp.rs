use log::warn;
use modular_bitfield::bitfield;
use modular_bitfield::prelude::*;

#[rustfmt::skip]
const RATE_TABLE: [u16; 32] = [
      0, 2048, 1536, 1280, 1024, 768, 640, 512,
    384,  320,  256,  192,  160, 128,  96,  80, 
     64,   48,   40,   32,   24,  20,  16,  12,
     10,    8,    6,    5,    4,   3,   2,   1,
];

pub struct Dsp {
    pub ram: [u8; 0x10000], // 64KB
    voice: [Voice; 8],

    master_volume: [i8; 2],   // 0x0C, 0x1C
    echo_volume: [i8; 2],     // 0x2C, 0x3C
    flag: Flags,              // 0x6C
    echo_feedback_volume: i8, // 0x0D
    na: u8,                   // 0x1D
    sample_table_address: u8, // 0x5D
    echo_buffer_address: u8,  // 0x6D
    echo_buffer_size: u8,     // 0x7D

    noise: Noise,

    audio_buffer: Vec<(i16, i16)>,
}

impl Dsp {
    pub fn tick(&mut self) {
        let noise = self.noise.generate_noise();
        for ch in 0..8 {
            let prev_voice = if ch > 0 {
                Some(self.voice[ch - 1].voice_params.sample)
            } else {
                None
            };
            self.voice[ch].tick(&self.ram, self.sample_table_address, prev_voice, noise);
        }

        let mut output = [0; 2];

        for i in 0..2 {
            let mut normal_voice = 0i32;

            // let mut ch_volume = [0; 8];
            for ch in 0..8 {
                let sample = ((self.voice[ch].voice_params.sample << 1) as i32) >> 1;
                let c = (sample * self.voice[ch].voice_params.volume[i] as i32) >> 6;
                // ch_volume[ch] = c;
                normal_voice = (normal_voice + c).clamp(-0x8000, 0x7FFF);
                // if self.voice[ch].voice_status.enable_echo {
                //     let echo = ((sample * self.voice[ch].voice_params.volume[i] as i32) >> 6) >> 1;
                //     output[i] += echo;
                // }
            }
            // for ch in 0..8 {
            //     print!("{:06}  ", ch_volume[ch]);
            // }
            // println!();

            normal_voice =
                ((normal_voice * self.master_volume[i] as i32) >> 7).clamp(-0x8000, 0x7FFF);

            output[i] = if self.flag.enable_mute() {
                0
            } else {
                normal_voice as i16
            };

            output[i] = !output[i];
        }

        self.audio_buffer.push((output[0], output[1]));
    }

    pub fn clear_audio_buffer(&mut self) {
        self.audio_buffer.clear();
    }

    pub fn get_audio_buffer(&self) -> &[(i16, i16)] {
        &self.audio_buffer
    }
}

#[bitfield(bits = 8)]
#[derive(Debug, Clone, Copy)]
struct Flags {
    noise_frequency: B5,
    disable_echo_buffer_write: bool,
    enable_mute: bool,
    enable_reset: bool,
}

impl Default for Flags {
    fn default() -> Self {
        Flags::new()
            .with_noise_frequency(0)
            .with_disable_echo_buffer_write(true)
            .with_enable_mute(true)
            .with_enable_reset(true)
    }
}

impl Default for Dsp {
    fn default() -> Self {
        Dsp {
            ram: [0; 0x10000],
            voice: [Voice::default(); 8],

            master_volume: [0; 2],
            echo_volume: [0; 2],
            flag: Default::default(),
            echo_feedback_volume: 0,
            na: 0,
            sample_table_address: 0,
            echo_buffer_address: 0,
            echo_buffer_size: 0,

            noise: Default::default(),

            audio_buffer: Vec::new(),
        }
    }
}

impl Dsp {
    pub fn read(&self, addr: u8) -> u8 {
        match addr & 0x7F {
            0x0C => self.master_volume[0] as u8,
            0x1C => self.master_volume[1] as u8,
            0x2C => self.echo_volume[0] as u8,
            0x3C => self.echo_volume[1] as u8,
            0x4C => {
                let mut ret = 0;
                for ch in 0..8 {
                    if self.voice[ch].voice_status.key_on {
                        ret |= 1 << ch;
                    }
                }
                ret
            }
            0x5C => {
                let mut ret = 0;
                for ch in 0..8 {
                    if self.voice[ch].voice_status.key_off {
                        ret |= 1 << ch;
                    }
                }
                ret
            }
            0x6C => self.flag.bytes[0],
            0x7C => {
                let mut ret = 0;
                for ch in 0..8 {
                    if self.voice[ch].voice_status.voice_end {
                        ret |= 1 << ch;
                    }
                }
                ret
            }
            0x0D => self.echo_feedback_volume as u8,
            0x1D => self.na,
            0x2D => {
                let mut ret = 0;
                for ch in 0..8 {
                    if self.voice[ch].voice_status.enable_pitch_modulation {
                        ret |= 1 << ch;
                    }
                }
                ret
            }
            0x3D => {
                let mut ret = 0;
                for ch in 0..8 {
                    if self.voice[ch].voice_status.enable_noise {
                        ret |= 1 << ch;
                    }
                }
                ret
            }
            0x4D => {
                let mut ret = 0;
                for ch in 0..8 {
                    if self.voice[ch].voice_status.enable_echo {
                        ret |= 1 << ch;
                    }
                }
                ret
            }
            0x5D => self.sample_table_address,
            0x6D => self.echo_buffer_address,
            0x7D => self.echo_buffer_size,
            _ => {
                let ch = ((addr >> 4) & 0x7) as usize;
                self.voice[ch].read(addr & 0xF)
            }
        }
    }

    pub fn write(&mut self, addr: u8, data: u8) {
        match addr & 0x7F {
            0x0C => self.master_volume[0] = data as i8,
            0x1C => self.master_volume[1] = data as i8,
            0x2C => self.echo_volume[0] = data as i8,
            0x3C => self.echo_volume[1] = data as i8,
            0x4C => {
                for ch in 0..8 {
                    self.voice[ch].voice_status.key_on = data & (1 << ch) != 0;
                }
            }
            0x5C => {
                for ch in 0..8 {
                    self.voice[ch].voice_status.key_off = data & (1 << ch) != 0;
                }
            }
            0x6C => {
                self.flag.bytes[0] = data;

                self.noise.set_frequency(self.flag.noise_frequency());
                if self.flag.enable_reset() {
                    for ch in 0..8 {
                        self.voice[ch].voice_status.key_off = true;
                        self.voice[ch].envelopes.envelope = 0;
                    }
                    self.flag.set_enable_reset(false);
                }
            }
            0x7C => {
                for ch in 0..8 {
                    self.voice[ch].voice_status.voice_end = false;
                }
            }
            0x0D => self.echo_feedback_volume = data as i8,
            0x1D => self.na = data,
            0x2D => {
                for ch in 0..8 {
                    self.voice[ch].voice_status.enable_pitch_modulation = data & (1 << ch) != 0;
                }
            }
            0x3D => {
                for ch in 0..8 {
                    self.voice[ch].voice_status.enable_noise = data & (1 << ch) != 0;
                }
            }
            0x4D => {
                for ch in 0..8 {
                    self.voice[ch].voice_status.enable_echo = data & (1 << ch) != 0;
                }
            }
            0x5D => self.sample_table_address = data,
            0x6D => self.echo_buffer_address = data,
            0x7D => self.echo_buffer_size = data,
            _ => {
                let ch = ((addr >> 4) & 0x7) as usize;
                self.voice[ch].write(addr & 0xF, data);
            }
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
struct Voice {
    voice_params: VoiceParams,
    voice_status: VoiceStatus,
    brr: BrrParams,
    brr_block: BrrBlock,
    envelopes: Envelopes,
    na: [u8; 3], // 0xXA, 0xXB, 0xXE
}

impl Voice {
    fn tick(&mut self, ram: &[u8], sample_table_address: u8, prev_voice: Option<i16>, noise: i16) {
        if self.voice_status.is_key_on() {
            self.envelopes.reset_envelope_on_key_on();
            self.voice_params.gaussian_sample_points.fill(0);
            self.set_brr_address(ram, sample_table_address, false);
            self.decode_brr(ram);
        }

        if self.voice_status.is_key_off() {
            self.envelopes.state = EnvelopeState::Release;
        }

        let mut step = self.voice_params.sample_rate & 0x3FFF;
        if self.voice_status.enable_pitch_modulation && prev_voice.is_some() {
            let factor = (prev_voice.unwrap() >> 4) + 0x400;
            step = ((step as i32 * factor as i32) >> 10) as u16;
            step = step.min(0x3FFF);
        }

        let mut prev_brr_index = ((self.brr.pitch_counter >> 12) & 0xF) as usize;
        let (counter, overflow) = self.brr.pitch_counter.overflowing_add(step);
        self.brr.pitch_counter = counter;

        if overflow {
            for i in prev_brr_index + 1..16 {
                self.voice_params.push_sample(self.brr_block.data[i]);
            }
            self.load_next_brr(ram, sample_table_address);
            prev_brr_index = 0;
        }

        let cur_brr_index = ((self.brr.pitch_counter >> 12) & 0xF) as usize;
        for i in prev_brr_index + 1..=cur_brr_index {
            self.voice_params.push_sample(self.brr_block.data[i]);
        }

        let sample = if self.voice_status.enable_noise {
            noise
        } else {
            let gaussian_index = ((counter >> 4) & 0xFF) as usize;
            self.apply_gaussian_interpolation(gaussian_index)
        };

        self.envelopes.update_envelope();
        // self.voice_params.sample = sample;
        self.voice_params.sample = ((sample as i32 * self.envelopes.envelope as i32) >> 11) as i16;
    }

    fn set_brr_address(&mut self, ram: &[u8], sample_table_address: u8, repeat: bool) {
        let table_address =
            (sample_table_address as u16 * 0x100 + self.brr.source_number as u16 * 4) as usize;
        self.brr.address = if repeat {
            u16::from_le_bytes(
                ram[table_address + 2..table_address + 4]
                    .try_into()
                    .unwrap(),
            )
        } else {
            u16::from_le_bytes(ram[table_address..table_address + 2].try_into().unwrap())
        };
        warn!(
            "Start BRR decode: entry = {table_address:04X}, addr = {:04X}",
            self.brr.address,
        );
    }

    fn load_next_brr(&mut self, ram: &[u8], sample_table_address: u8) {
        // if self.brr_block.header.end() {
        //     if self.brr_block.header.repeat() {
        //         self.set_brr_address(ram, sample_table_address, true);
        //         self.decode_brr(ram);
        //     } else {
        //         self.envelopes.state = EnvelopeState::Release;
        //         self.envelopes.envelope = 0;
        //         self.set_brr_address(ram, sample_table_address, true);
        //         self.decode_brr(ram);
        //     }
        // } else {
        //     self.decode_brr(ram);
        // }
        if !self.brr_block.header.end() {
            self.decode_brr(ram);
        } else if self.brr_block.header.repeat() {
            self.set_brr_address(ram, sample_table_address, true);
            self.decode_brr(ram);
        } else {
            self.envelopes.state = EnvelopeState::Release;
            self.envelopes.envelope = 0;
            self.set_brr_address(ram, sample_table_address, true);
            self.decode_brr(ram);
        }
    }

    fn decode_brr(&mut self, ram: &[u8]) {
        // warn!("Decode BRR block: {:04X}", self.brr.address);
        warn!(
            "Decode BRR block: {:04X}, data = {:0X}",
            self.brr.address, ram[self.brr.address as usize]
        );
        let header = BrrBlockHeader::from_bytes([ram[self.brr.address as usize]]);
        // let header = BrrBlockHeader::from_bytes([ram[self.brr_cur_addr as usize]]);
        self.brr.address = self.brr.address.wrapping_add(1);

        warn!(
            "BRR header: end = {}, repeat = {}, filter = {}, shift = {}",
            header.end(),
            header.repeat(),
            header.filter_num(),
            header.shift()
        );

        if header.end() {
            self.voice_status.voice_end = true;
        }

        let mut data = [0; 16];
        for i in 0..16 {
            let nibble = ram[self.brr.address as usize] >> ((i & 1 ^ 1) * 4);
            let nibble = ((nibble as i16) << 12) >> 12;
            if i & 1 == 1 {
                self.brr.address = self.brr.address.wrapping_add(1);
            }

            let sample = if header.shift() <= 12 {
                (nibble << header.shift()) >> 1
            } else {
                ((nibble >> 3) << 12) >> 1
            } as i32;

            let old = self.voice_params.old as i32;
            let older = self.voice_params.older as i32;

            let new = match header.filter_num() {
                0 => sample,
                1 => sample + old + ((-old) >> 4),
                2 => sample + old * 2 + ((-old * 3) >> 5) - older + (older >> 4),
                3 => sample + old * 2 + ((-old * 13) >> 6) - older + ((older * 3) >> 4),
                _ => unreachable!(),
            };

            let new = new.clamp(-0x8000, 0x7FFF) as i16;
            self.voice_params.older = self.voice_params.old;
            self.voice_params.old = new;
            data[i] = new;
        }
        warn!("Brr data: {:?}", data);
        self.brr_block = BrrBlock { header, data };
        self.voice_params.push_sample(data[0]);
    }

    fn apply_gaussian_interpolation(&self, index: usize) -> i16 {
        let p3 = ((self.voice_params.gaussian_sample_points[3] as i32
            * GAUSS_TABLE[0xFF - index] as i32)
            >> 10) as i16;
        let p2 = ((self.voice_params.gaussian_sample_points[2] as i32
            * GAUSS_TABLE[0x1FF - index] as i32)
            >> 10) as i16;
        let p1 = ((self.voice_params.gaussian_sample_points[1] as i32
            * GAUSS_TABLE[0x100 + index] as i32)
            >> 10) as i16;
        let p0 = ((self.voice_params.gaussian_sample_points[0] as i32 * GAUSS_TABLE[index] as i32)
            >> 10) as i16;

        let mut output = p3.wrapping_add(p2);
        output = output.wrapping_add(p1);
        output = output.saturating_add(p0);
        output >> 1
    }
}

impl Voice {
    fn read(&self, addr: u8) -> u8 {
        match addr {
            0x0 => self.voice_params.volume[0] as u8,
            0x1 => self.voice_params.volume[1] as u8,
            0x2 => self.voice_params.sample_rate as u8,
            0x3 => (self.voice_params.sample_rate >> 8) as u8,
            0x4 => self.brr.source_number,
            0x5 => self.envelopes.adsr_settings.bytes[0],
            0x6 => self.envelopes.adsr_settings.bytes[1],
            0x7 => self.envelopes.gain_settings,
            0x8 => (self.envelopes.envelope >> 4) as u8,
            0x9 => (self.voice_params.sample >> 7) as u8,
            0xA => self.na[0],
            0xB => self.na[1],
            0xE => self.na[2],
            0xF => self.voice_params.fir_coefficient as u8,
            _ => unreachable!(),
        }
    }

    fn write(&mut self, addr: u8, data: u8) {
        match addr {
            0x0 => self.voice_params.volume[0] = data as i8,
            0x1 => self.voice_params.volume[1] = data as i8,
            0x2 => {
                self.voice_params.sample_rate =
                    (self.voice_params.sample_rate & 0xFF00) | data as u16
            }
            0x3 => {
                self.voice_params.sample_rate =
                    (self.voice_params.sample_rate & 0x00FF) | (data as u16) << 8
            }
            0x4 => self.brr.source_number = data,
            0x5 => self.envelopes.adsr_settings.bytes[0] = data,
            0x6 => self.envelopes.adsr_settings.bytes[1] = data,
            0x7 => self.envelopes.gain_settings = data,
            0x8 => self.envelopes.envelope = (data as u16) << 4,
            0x9 => self.voice_params.sample = ((data as u16) << 7) as i16,
            0xA => self.na[0] = data,
            0xB => self.na[1] = data,
            0xE => self.na[2] = data,
            0xF => self.voice_params.fir_coefficient = data as i8,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
struct VoiceParams {
    volume: [i8; 2],     // 0xX0, 0xX1
    sample_rate: u16,    // 0xX2, 0xX3
    sample: i16,         // 0xX9
    fir_coefficient: i8, // 0xXF
    gaussian_sample_points: [i16; 4],
    old: i16,
    older: i16,
}

impl VoiceParams {
    fn push_sample(&mut self, sample: i16) {
        self.gaussian_sample_points[3] = self.gaussian_sample_points[2];
        self.gaussian_sample_points[2] = self.gaussian_sample_points[1];
        self.gaussian_sample_points[1] = self.gaussian_sample_points[0];
        self.gaussian_sample_points[0] = sample;
    }
}

#[derive(Debug, Copy, Clone)]
struct VoiceStatus {
    key_on: bool,                  // 0x4C
    key_off: bool,                 // 0x5C
    voice_end: bool,               // 0x7C
    enable_pitch_modulation: bool, // 0x2D
    enable_noise: bool,            // 0x3D,
    enable_echo: bool,             // 0x4D
}

impl Default for VoiceStatus {
    fn default() -> Self {
        VoiceStatus {
            voice_end: true,
            key_off: false,
            key_on: false,
            enable_pitch_modulation: false,
            enable_noise: false,
            enable_echo: false,
        }
    }
}

impl VoiceStatus {
    fn is_key_on(&mut self) -> bool {
        let ret = self.key_on;
        self.key_on = false;
        ret
    }

    fn is_key_off(&mut self) -> bool {
        let ret = self.key_off;
        self.key_off = false;
        ret
    }
}

#[derive(Debug, Default, Copy, Clone)]
struct BrrBlock {
    header: BrrBlockHeader,
    data: [i16; 16],
}

#[bitfield(bits = 8)]
#[derive(Default, Debug, Clone, Copy)]
struct BrrBlockHeader {
    end: bool,
    repeat: bool,
    filter_num: B2,
    shift: B4,
}

#[derive(Debug, Default, Clone, Copy)]
struct BrrParams {
    source_number: u8, // 0xX4
    pitch_counter: u16,
    address: u16,
}

#[derive(Debug, Default, Clone, Copy)]
struct Envelopes {
    adsr_settings: AdsrSettings, // 0xX5, 0xX6
    gain_settings: u8,           // 0xX7
    envelope: u16,               // 0xX8
    counter: u16,
    state: EnvelopeState,
}

impl Envelopes {
    fn reset_envelope_on_key_on(&mut self) {
        self.envelope = 0;
        self.counter = 0;
        self.state = EnvelopeState::Attack;
    }

    fn update_envelope(&mut self) {
        self.envelope &= 0x7FF;
        if self.state != EnvelopeState::Release && !self.adsr_settings.use_adsr() {
            self.update_gain_envelope();
        } else {
            self.update_adsr_envelope();
        }
    }

    fn update_gain_envelope(&mut self) {
        if self.gain_settings & 0x80 == 0 {
            self.envelope = (self.gain_settings & 0x7F) as u16 * 16;
        } else {
            let rate = self.gain_settings & 0x1F;
            let mode = (self.gain_settings >> 5) & 3;

            self.counter += 1;

            if rate == 0 {
                self.counter = 0;
            } else if self.counter >= RATE_TABLE[rate as usize] {
                self.counter = 0;

                match mode {
                    0 => self.envelope = self.envelope.saturating_sub(32),
                    1 => {
                        if self.envelope > 0 {
                            let step = ((self.envelope - 1) >> 8) + 1;
                            self.envelope = self.envelope.saturating_sub(step);
                        }
                    }
                    2 => self.envelope = (self.envelope + 32).min(0x7FF),
                    3 => {
                        if self.envelope < 0x600 {
                            self.envelope += 32;
                        } else {
                            self.envelope = (self.envelope + 8).min(0x7FF)
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    fn update_adsr_envelope(&mut self) {
        match self.state {
            EnvelopeState::Attack => self.process_attack(),
            EnvelopeState::Decay => self.process_decay(),
            EnvelopeState::Sustain => self.process_sustain(),
            EnvelopeState::Release => self.process_release(),
        }
    }

    fn process_attack(&mut self) {
        let rate = self.adsr_settings.attack_rate() * 2 + 1;
        self.counter += 1;
        if self.counter >= RATE_TABLE[rate as usize] {
            self.counter = 0;
            self.envelope = (self.envelope + if rate != 31 { 32 } else { 1024 }).min(0x7FF);
            if self.envelope >= 0x7E0 {
                self.state = EnvelopeState::Decay;
            }
        }
    }

    fn process_decay(&mut self) {
        let rate = self.adsr_settings.decay_rate() * 2 + 16;
        self.counter += 1;
        if self.counter >= RATE_TABLE[rate as usize] {
            self.counter = 0;
            let step = ((self.envelope - 1) >> 8) + 1;
            self.envelope -= step;
        }
        let boundary = (self.adsr_settings.sustain_level() as u16 + 1) * 0x100;
        if self.envelope <= boundary {
            self.state = EnvelopeState::Sustain;
        }
    }

    fn process_sustain(&mut self) {
        let rate = self.adsr_settings.sustain_rate();
        if rate == 0 {
            self.counter = 0;
        } else {
            self.counter += 1;
            if self.counter >= RATE_TABLE[rate as usize] {
                self.counter = 0;
                if self.envelope > 0 {
                    let step = ((self.envelope - 1) >> 8) + 1;
                    self.envelope -= step;
                }
            }
        }
    }

    fn process_release(&mut self) {
        self.counter += 1;
        if self.counter >= RATE_TABLE[31] {
            self.counter = 0;
            self.envelope = self.envelope.saturating_sub(8);
        }
    }
}

#[bitfield(bits = 16)]
#[derive(Debug, Default, Clone, Copy)]
struct AdsrSettings {
    attack_rate: B4,
    decay_rate: B3,
    use_adsr: bool,
    sustain_rate: B5,
    sustain_level: B3,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum EnvelopeState {
    #[default]
    Attack,
    Decay,
    Sustain,
    Release,
}

struct Noise {
    noise: i16,
    frequency: usize,
    counter: u16,
}

impl Default for Noise {
    fn default() -> Self {
        Noise {
            noise: 1,
            frequency: 0,
            counter: 0,
        }
    }
}

impl Noise {
    fn generate_noise(&mut self) -> i16 {
        let rate = self.frequency;

        if rate == 0 {
            self.counter = 0;
            return self.noise;
        }

        self.counter += 1;
        if self.counter >= RATE_TABLE[rate] {
            self.counter = 0;
            let b0 = self.noise & 1;
            let b1 = (self.noise >> 1) & 1;
            self.noise = (b0 ^ b1) << 14 | ((self.noise >> 1) & 0x3FFF);
        }
        self.noise
    }

    fn set_frequency(&mut self, frequency: u8) {
        self.frequency = frequency as usize;
    }
}

#[rustfmt::skip]
const GAUSS_TABLE: [u16; 512] = [
    0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000,
    0x001, 0x001, 0x001, 0x001, 0x001, 0x001, 0x001, 0x001, 0x001, 0x001, 0x001, 0x002, 0x002, 0x002, 0x002, 0x002,
    0x002, 0x002, 0x003, 0x003, 0x003, 0x003, 0x003, 0x004, 0x004, 0x004, 0x004, 0x004, 0x005, 0x005, 0x005, 0x005,
    0x006, 0x006, 0x006, 0x006, 0x007, 0x007, 0x007, 0x008, 0x008, 0x008, 0x009, 0x009, 0x009, 0x00A, 0x00A, 0x00A,
    0x00B, 0x00B, 0x00B, 0x00C, 0x00C, 0x00D, 0x00D, 0x00E, 0x00E, 0x00F, 0x00F, 0x00F, 0x010, 0x010, 0x011, 0x011,
    0x012, 0x013, 0x013, 0x014, 0x014, 0x015, 0x015, 0x016, 0x017, 0x017, 0x018, 0x018, 0x019, 0x01A, 0x01B, 0x01B,
    0x01C, 0x01D, 0x01D, 0x01E, 0x01F, 0x020, 0x020, 0x021, 0x022, 0x023, 0x024, 0x024, 0x025, 0x026, 0x027, 0x028,
    0x029, 0x02A, 0x02B, 0x02C, 0x02D, 0x02E, 0x02F, 0x030, 0x031, 0x032, 0x033, 0x034, 0x035, 0x036, 0x037, 0x038,
    0x03A, 0x03B, 0x03C, 0x03D, 0x03E, 0x040, 0x041, 0x042, 0x043, 0x045, 0x046, 0x047, 0x049, 0x04A, 0x04C, 0x04D,
    0x04E, 0x050, 0x051, 0x053, 0x054, 0x056, 0x057, 0x059, 0x05A, 0x05C, 0x05E, 0x05F, 0x061, 0x063, 0x064, 0x066,
    0x068, 0x06A, 0x06B, 0x06D, 0x06F, 0x071, 0x073, 0x075, 0x076, 0x078, 0x07A, 0x07C, 0x07E, 0x080, 0x082, 0x084,
    0x086, 0x089, 0x08B, 0x08D, 0x08F, 0x091, 0x093, 0x096, 0x098, 0x09A, 0x09C, 0x09F, 0x0A1, 0x0A3, 0x0A6, 0x0A8,
    0x0AB, 0x0AD, 0x0AF, 0x0B2, 0x0B4, 0x0B7, 0x0BA, 0x0BC, 0x0BF, 0x0C1, 0x0C4, 0x0C7, 0x0C9, 0x0CC, 0x0CF, 0x0D2,
    0x0D4, 0x0D7, 0x0DA, 0x0DD, 0x0E0, 0x0E3, 0x0E6, 0x0E9, 0x0EC, 0x0EF, 0x0F2, 0x0F5, 0x0F8, 0x0FB, 0x0FE, 0x101,
    0x104, 0x107, 0x10B, 0x10E, 0x111, 0x114, 0x118, 0x11B, 0x11E, 0x122, 0x125, 0x129, 0x12C, 0x130, 0x133, 0x137,
    0x13A, 0x13E, 0x141, 0x145, 0x148, 0x14C, 0x150, 0x153, 0x157, 0x15B, 0x15F, 0x162, 0x166, 0x16A, 0x16E, 0x172,
    0x176, 0x17A, 0x17D, 0x181, 0x185, 0x189, 0x18D, 0x191, 0x195, 0x19A, 0x19E, 0x1A2, 0x1A6, 0x1AA, 0x1AE, 0x1B2,
    0x1B7, 0x1BB, 0x1BF, 0x1C3, 0x1C8, 0x1CC, 0x1D0, 0x1D5, 0x1D9, 0x1DD, 0x1E2, 0x1E6, 0x1EB, 0x1EF, 0x1F3, 0x1F8,
    0x1FC, 0x201, 0x205, 0x20A, 0x20F, 0x213, 0x218, 0x21C, 0x221, 0x226, 0x22A, 0x22F, 0x233, 0x238, 0x23D, 0x241,
    0x246, 0x24B, 0x250, 0x254, 0x259, 0x25E, 0x263, 0x267, 0x26C, 0x271, 0x276, 0x27B, 0x280, 0x284, 0x289, 0x28E,
    0x293, 0x298, 0x29D, 0x2A2, 0x2A6, 0x2AB, 0x2B0, 0x2B5, 0x2BA, 0x2BF, 0x2C4, 0x2C9, 0x2CE, 0x2D3, 0x2D8, 0x2DC,
    0x2E1, 0x2E6, 0x2EB, 0x2F0, 0x2F5, 0x2FA, 0x2FF, 0x304, 0x309, 0x30E, 0x313, 0x318, 0x31D, 0x322, 0x326, 0x32B,
    0x330, 0x335, 0x33A, 0x33F, 0x344, 0x349, 0x34E, 0x353, 0x357, 0x35C, 0x361, 0x366, 0x36B, 0x370, 0x374, 0x379,
    0x37E, 0x383, 0x388, 0x38C, 0x391, 0x396, 0x39B, 0x39F, 0x3A4, 0x3A9, 0x3AD, 0x3B2, 0x3B7, 0x3BB, 0x3C0, 0x3C5,
    0x3C9, 0x3CE, 0x3D2, 0x3D7, 0x3DC, 0x3E0, 0x3E5, 0x3E9, 0x3ED, 0x3F2, 0x3F6, 0x3FB, 0x3FF, 0x403, 0x408, 0x40C,
    0x410, 0x415, 0x419, 0x41D, 0x421, 0x425, 0x42A, 0x42E, 0x432, 0x436, 0x43A, 0x43E, 0x442, 0x446, 0x44A, 0x44E,
    0x452, 0x455, 0x459, 0x45D, 0x461, 0x465, 0x468, 0x46C, 0x470, 0x473, 0x477, 0x47A, 0x47E, 0x481, 0x485, 0x488,
    0x48C, 0x48F, 0x492, 0x496, 0x499, 0x49C, 0x49F, 0x4A2, 0x4A6, 0x4A9, 0x4AC, 0x4AF, 0x4B2, 0x4B5, 0x4B7, 0x4BA,
    0x4BD, 0x4C0, 0x4C3, 0x4C5, 0x4C8, 0x4CB, 0x4CD, 0x4D0, 0x4D2, 0x4D5, 0x4D7, 0x4D9, 0x4DC, 0x4DE, 0x4E0, 0x4E3,
    0x4E5, 0x4E7, 0x4E9, 0x4EB, 0x4ED, 0x4EF, 0x4F1, 0x4F3, 0x4F5, 0x4F6, 0x4F8, 0x4FA, 0x4FB, 0x4FD, 0x4FF, 0x500,
    0x502, 0x503, 0x504, 0x506, 0x507, 0x508, 0x50A, 0x50B, 0x50C, 0x50D, 0x50E, 0x50F, 0x510, 0x511, 0x511, 0x512,
    0x513, 0x514, 0x514, 0x515, 0x516, 0x516, 0x517, 0x517, 0x517, 0x518, 0x518, 0x518, 0x518, 0x518, 0x519, 0x519,
];
