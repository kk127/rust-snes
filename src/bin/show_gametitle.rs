fn main() {
    let rom_path = std::env::args()
        .nth(1)
        .expect("Usage: bin/show_gametitle <path-to-rom>");
    let rom = std::fs::read(rom_path).expect("Failed to read ROM file");
    for title in get_title(&rom) {
        println!("{}", title);
        println!("len: {}", title.len());
    }
}

fn get_title(rom: &[u8]) -> Vec<String> {
    let mut ret = Vec::new();
    let base_addr = [0x007F00, 0x00FF00, 0x40FF00];
    for &base in base_addr.iter() {
        println!("base: {:x}", base);
        if base + 0x100 > rom.len() {
            ret.push("over".to_string());
            continue;
        }

        let tmp = match std::str::from_utf8(&rom[base + 0xC0..base + 0xC0 + 21]) {
            Ok(title) => title.trim().to_string(),
            Err(_) => "None".to_string(),
        };
        ret.push(tmp);
    }

    ret
}
