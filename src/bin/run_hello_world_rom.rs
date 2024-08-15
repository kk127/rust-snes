use rust_snes::Snes;
fn main() {
    let rom_path = std::env::args()
        .nth(1)
        .expect("Usage: bin/run_hello_world_rom <path-to-rom>");
    let rom = std::fs::read(rom_path).expect("Failed to read ROM file");
    let mut snes = Snes::new(rom);
    snes.run();
}
