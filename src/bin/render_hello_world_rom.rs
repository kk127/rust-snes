use log;
use rust_snes::Snes;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use std::time::Duration;
fn main() -> Result<(), String> {
    env_logger::init();
    let rom_path = std::env::args()
        .nth(1)
        .expect("Usage: bin/run_hello_world_rom <path-to-rom>");
    let rom = std::fs::read(rom_path).expect("Failed to read ROM file");
    let mut snes = Snes::new(rom);

    let sdl2_context = sdl2::init()?;
    let video_subsystem = sdl2_context.video()?;

    let window = video_subsystem
        .window("rust-snes", 256, 224)
        .position_centered()
        .resizable()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    let mut event_pump = sdl2_context.event_pump()?;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    break 'running; // ループを終了してプログラムを終了
                }
                _ => {}
            }
        }

        // 背景を黒でクリア
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        snes.exec_frame();
        let screen = snes.context.inner1.inner2.ppu.screen;
        // println!("screen: {:?}", screen);
        for x in 0..256 {
            for y in 0..224 {
                let color = screen[y * 256 + x];
                // println!("color: {}", color);
                let mut r = color & 0x1F;
                let mut g = (color >> 5) & 0x1F;
                let mut b = (color >> 10) & 0x1F;
                r = r << 3 | r >> 2;
                g = g << 3 | g >> 2;
                b = b << 3 | b >> 2;
                // println!("b: {}, g: {}, r: {}", b, g, r);
                canvas.set_draw_color(Color::RGB(r as u8, g as u8, b as u8));
                canvas.draw_point((x as i32, y as i32)).unwrap();
            }
        }

        // 描画をウィンドウに反映
        canvas.present();

        // 16ms待機して約60FPSを維持
        // std::thread::sleep(Duration::from_millis(1000));
        std::thread::sleep(Duration::from_millis(16));
    }

    // loop {
    // snes.exec_frame();
    // let screen = snes.context.inner1.inner2.ppu.screen;
    // }
    Ok(())
}