use log;
use rust_snes::{Key, Snes};
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

    // ウィンドウサイズを512x448に変更
    let window = video_subsystem
        .window("rust-snes", 256 * 3, 224 * 3)
        .position_centered()
        .resizable()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();

    // 論理的な描画サイズを256x224に設定
    canvas.set_logical_size(256, 224).unwrap();

    // GameControllerのサブシステムを取得
    let game_controller_subsystem = sdl2_context.game_controller()?;

    // 接続されているコントローラーを探す
    let available = game_controller_subsystem
        .num_joysticks()
        .map_err(|e| e.to_string())?;
    let mut controller = None;

    for id in 0..available {
        if game_controller_subsystem.is_game_controller(id) {
            controller = Some(game_controller_subsystem.open(id).unwrap());
            println!(
                "Controller detected: {}",
                controller.as_ref().unwrap().name()
            );
            break;
        }
    }

    // コントローラーが見つからなければエラー
    let controller = match controller {
        Some(c) => c,
        None => {
            println!("No game controller found!");
            return Ok(());
        }
    };

    let mut event_pump = sdl2_context.event_pump()?;

    let mut keys = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    'running: loop {
        let start_time = std::time::Instant::now();
        for event in event_pump.poll_iter() {
            match event {
                // 押されたkeyに対応するkeyを取得し、keysに追加
                Event::Quit { .. } => break 'running,
                // コントローラのボタンが押されたとき
                Event::ControllerButtonDown { button, .. } => match button {
                    sdl2::controller::Button::Y => keys[0].push(Key::X),
                    sdl2::controller::Button::B => keys[0].push(Key::A),
                    sdl2::controller::Button::A => keys[0].push(Key::B),
                    sdl2::controller::Button::X => keys[0].push(Key::Y),
                    sdl2::controller::Button::Start => keys[0].push(Key::Start),
                    sdl2::controller::Button::Back => keys[0].push(Key::Select),
                    sdl2::controller::Button::DPadUp => keys[0].push(Key::Up),
                    sdl2::controller::Button::DPadDown => keys[0].push(Key::Down),
                    sdl2::controller::Button::DPadLeft => keys[0].push(Key::Left),
                    sdl2::controller::Button::DPadRight => keys[0].push(Key::Right),
                    sdl2::controller::Button::LeftShoulder => keys[0].push(Key::L),
                    sdl2::controller::Button::RightShoulder => keys[0].push(Key::R),
                    _ => {}
                },
                Event::ControllerButtonUp { button, .. } => match button {
                    sdl2::controller::Button::Y => keys[0].retain(|&k| k != Key::X),
                    sdl2::controller::Button::B => keys[0].retain(|&k| k != Key::A),
                    sdl2::controller::Button::A => keys[0].retain(|&k| k != Key::B),
                    sdl2::controller::Button::X => keys[0].retain(|&k| k != Key::Y),
                    sdl2::controller::Button::Start => keys[0].retain(|&k| k != Key::Start),
                    sdl2::controller::Button::Back => keys[0].retain(|&k| k != Key::Select),
                    sdl2::controller::Button::DPadUp => keys[0].retain(|&k| k != Key::Up),
                    sdl2::controller::Button::DPadDown => keys[0].retain(|&k| k != Key::Down),
                    sdl2::controller::Button::DPadLeft => keys[0].retain(|&k| k != Key::Left),
                    sdl2::controller::Button::DPadRight => keys[0].retain(|&k| k != Key::Right),
                    sdl2::controller::Button::LeftShoulder => keys[0].retain(|&k| k != Key::L),
                    sdl2::controller::Button::RightShoulder => keys[0].retain(|&k| k != Key::R),
                    _ => {}
                },
                _ => {}
            }
        }
        // if !keys[0].is_empty() {
        //     println!("{:?}", keys);
        // }
        snes.set_keys(keys.clone());

        // 背景を黒でクリア
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        snes.exec_frame();
        let screen = snes.context.inner1.inner2.ppu.frame;

        for x in 0..256 {
            for y in 0..224 {
                let color = screen[y * 256 + x];
                let mut r = color & 0x1F;
                let mut g = (color >> 5) & 0x1F;
                let mut b = (color >> 10) & 0x1F;
                r = r << 3 | r >> 2;
                g = g << 3 | g >> 2;
                b = b << 3 | b >> 2;
                canvas.set_draw_color(Color::RGB(r as u8, g as u8, b as u8));

                // 倍のウィンドウサイズに描画するためのスケーリング
                canvas.draw_point((x as i32, y as i32)).unwrap();
            }
        }

        // 描画をウィンドウに反映
        canvas.present();

        // 16ms待機して約60FPSを維持
        // std::thread::sleep(Duration::from_millis(16));
        let elapsed = start_time.elapsed();
        if elapsed < Duration::from_millis(16) {
            std::thread::sleep(Duration::from_millis(16) - elapsed);
        }
    }

    Ok(())
}
