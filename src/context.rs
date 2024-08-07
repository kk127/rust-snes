pub trait Bus {
    fn bus_read(&self, addr: u16) -> u8;
    fn bus_write(&mut self, addr: u16, data: u8);
}

pub trait Ppu {
    fn ppu_read(&self, addr: u16) -> u8;
    fn ppu_write(&mut self, addr: u16, data: u8);
}

pub trait Timing {
    fn elapse(&mut self, clock: u64);
}
