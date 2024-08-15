use crate::context;
trait Context: context::Timing {}
impl<T: context::Timing> Context for T {}
pub struct Ppu {}

impl Default for Ppu {
    fn default() -> Ppu {
        Ppu {}
    }
}

impl Ppu {
    pub fn read(&mut self, addr: u32, ctx: &mut impl Context) -> u8 {
        println!("ppu read: {:x}", addr);
        0
    }

    pub fn write(&mut self, addr: u32, data: u8, ctx: &mut impl Context) {
        println!("ppu write: {:x}, {:x}", addr, data);
    }
}
