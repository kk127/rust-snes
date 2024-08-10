pub trait Bus {
    fn bus_read(&self, addr: u32) -> u8;
    fn bus_write(&mut self, addr: u32, data: u8);
}

pub trait Ppu {
    fn ppu_read(&self, addr: u32) -> u8;
    fn ppu_write(&mut self, addr: u32, data: u8);
}

pub trait Timing {
    fn elapse(&mut self, clock: u64);
}
