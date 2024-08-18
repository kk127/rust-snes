fn main() {
    let rom_path = std::env::args()
        .nth(1)
        .expect("Usage: bin/check_sum <path-to-rom>");
    let rom = std::fs::read(rom_path).expect("Failed to read ROM file");
    let base = [0x007F00, 0x00FF00, 0x40FF00];
    let calc_checksum = calc_checksum(&rom);
    for &base in base.iter() {
        if base + 0x100 > rom.len() {
            println!("over");
            println!();
            continue;
        }
        let check_sum_comp =
            u16::from_le_bytes(rom[base + 0xDC..base + 0xDC + 2].try_into().unwrap());
        let checksum = u16::from_le_bytes(rom[base + 0xDE..base + 0xDE + 2].try_into().unwrap());
        println!("calc_checksum:  {:016b}", calc_checksum);
        println!("check_sum_comp: {:016b}", check_sum_comp);
        println!("checksum:       {:016b}", checksum);
        println!();
    }
}

fn calc_checksum(rom: &[u8]) -> u16 {
    let mut sum: u16 = 0;
    for i in 0..rom.len() {
        sum = sum.wrapping_add(rom[i] as u16);
    }
    sum
}
