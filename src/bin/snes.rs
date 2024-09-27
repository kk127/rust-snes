use anyhow::{Context, Result};
use dirs::data_dir;
use log::info;
use rust_snes::{Key, Snes};
use sdl2::audio;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

fn main() -> Result<()> {
    env_logger::init();

    // let rom_path = std::env::args()
    //     .nth(1)
    //     .context("Usage: --bin snes -- <path-to-rom>")?;
    // let rom = std::fs::read(&rom_path).context("Failed to read ROM file")?;
    // // rom_nameを取得。ただし、拡張子を除く
    // let binding = PathBuf::from(&rom_path);
    // let rom_name = binding
    //     .file_stem()
    //     .and_then(|s| s.to_str())
    //     .context("Failed to get the file name")?;
    // コマンドライン引数を取得
    let rom_arg = std::env::args()
        .nth(1)
        .context("Usage: --bin snes -- <path-to-rom>")?;

    // 引数をPathBufに変換
    let rom_path = PathBuf::from(rom_arg);

    // ROMファイルを読み込む
    let rom = std::fs::read(&rom_path).context("Failed to read ROM file")?;

    // rom_nameを取得（拡張子なし）
    let rom_name = rom_path
        .file_stem()
        .and_then(|s| s.to_str())
        .context("Failed to get the file name")?;

    // セーブデータをロード
    let backup = load_save_data(rom_name)?;

    let mut snes = Snes::new(rom, backup);

    let sdl2_context = sdl2::init()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to initialize SDL2")?;
    let video_subsystem = sdl2_context
        .video()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to initialize SDL2 video subsystem")?;

    // ウィンドウサイズを512x448に変更
    let window = video_subsystem
        .window("rust-snes", 256 * 3, 224 * 3)
        .position_centered()
        .resizable()
        .build()
        .context("Failed to create window")?;

    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .context("Failed to create canvas")?;

    // 論理的な描画サイズを256x224に設定
    canvas
        .set_logical_size(256, 224)
        .context("Failed to set logical size")?;

    let audio_subsystem = sdl2_context
        .audio()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to initialize SDL2 audio subsystem")?;
    let desired_spec = sdl2::audio::AudioSpecDesired {
        freq: Some(32_000),
        channels: Some(2),
        samples: Some(1024),
    };
    let audio_queue = audio_subsystem
        .open_queue::<i16, _>(None, &desired_spec)
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to open audio queue")?;
    audio_queue
        .queue_audio(&vec![0i16; 1024])
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to queue audio")?;
    audio_queue.resume();

    // GameControllerのサブシステムを取得
    let game_controller_subsystem = sdl2_context
        .game_controller()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to initialize SDL2 game controller subsystem")?;

    // 接続されているコントローラーを探す
    let available = game_controller_subsystem
        .num_joysticks()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to get the number of joysticks")?;

    let mut controller = None;

    for id in 0..available {
        if game_controller_subsystem.is_game_controller(id) {
            controller = Some(
                game_controller_subsystem
                    .open(id)
                    .context("Failed to open game controller")?,
            );
            println!(
                "Controller detected: {}",
                controller.as_ref().unwrap().name()
            );
            break;
        }
    }

    // コントローラーが見つからなければ、logに残す
    // let controller = controller.context("No game controller found!")?;

    let mut event_pump = sdl2_context
        .event_pump()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to get SDL2 event pump")?;

    let mut keys = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut frame = 0;
    'running: loop {
        let start_time = std::time::Instant::now();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
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
                // キーボードのキー押下処理を追加
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    sdl2::keyboard::Keycode::Up => keys[0].push(Key::Up),
                    sdl2::keyboard::Keycode::Down => keys[0].push(Key::Down),
                    sdl2::keyboard::Keycode::Left => keys[0].push(Key::Left),
                    sdl2::keyboard::Keycode::Right => keys[0].push(Key::Right),
                    sdl2::keyboard::Keycode::X => keys[0].push(Key::A),
                    sdl2::keyboard::Keycode::Z => keys[0].push(Key::B),
                    sdl2::keyboard::Keycode::S => keys[0].push(Key::X),
                    sdl2::keyboard::Keycode::A => keys[0].push(Key::Y),
                    sdl2::keyboard::Keycode::Q => keys[0].push(Key::L),
                    sdl2::keyboard::Keycode::W => keys[0].push(Key::R),
                    sdl2::keyboard::Keycode::Return => keys[0].push(Key::Start),
                    sdl2::keyboard::Keycode::LShift => keys[0].push(Key::Select),
                    _ => {}
                },
                // キーボードのキー離上げ処理を追加
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    sdl2::keyboard::Keycode::Up => keys[0].retain(|&k| k != Key::Up),
                    sdl2::keyboard::Keycode::Down => keys[0].retain(|&k| k != Key::Down),
                    sdl2::keyboard::Keycode::Left => keys[0].retain(|&k| k != Key::Left),
                    sdl2::keyboard::Keycode::Right => keys[0].retain(|&k| k != Key::Right),
                    sdl2::keyboard::Keycode::X => keys[0].retain(|&k| k != Key::A),
                    sdl2::keyboard::Keycode::Z => keys[0].retain(|&k| k != Key::B),
                    sdl2::keyboard::Keycode::S => keys[0].retain(|&k| k != Key::X),
                    sdl2::keyboard::Keycode::A => keys[0].retain(|&k| k != Key::Y),
                    sdl2::keyboard::Keycode::Q => keys[0].retain(|&k| k != Key::L),
                    sdl2::keyboard::Keycode::W => keys[0].retain(|&k| k != Key::R),
                    sdl2::keyboard::Keycode::Return => keys[0].retain(|&k| k != Key::Start),
                    sdl2::keyboard::Keycode::LShift => keys[0].retain(|&k| k != Key::Select),
                    _ => {}
                },
                _ => {}
            }
        }

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
                canvas
                    .draw_point((x as i32, y as i32))
                    .map_err(|e| anyhow::anyhow!(e))
                    .context("Failed to draw point")?;
            }
        }

        // 描画をウィンドウに反映
        canvas.present();

        let audio_buffer = snes.context.inner1.inner2.spc.audio_buffer();
        // println!("audio_buffer len: {:?}", audio_buffer.len());
        while audio_queue.size() > 1024 * 4 {
            std::thread::sleep(Duration::from_millis(1));
        }
        audio_queue
            .queue_audio(
                &audio_buffer
                    .iter()
                    .flat_map(|s| [s.0, s.1])
                    .collect::<Vec<i16>>(),
            )
            .unwrap();

        // 16ms待機して約60FPSを維持
        let elapsed = start_time.elapsed();
        if elapsed < Duration::from_millis(16) {
            std::thread::sleep(Duration::from_millis(16) - elapsed);
        }

        frame += 1;
        // セーブデータを保存
        if frame % 3600 == 0 {
            if let Some(data) = snes.backup() {
                info!("Saving data ...");
                save_data(rom_name, &data)?;
            }
        }
    }

    Ok(())
}

// Save save data
fn save_data(rom_name: &str, sram_data: &[u8]) -> Result<()> {
    // Retrieve application data directory and change to "rust-snes"
    let mut save_dir = data_dir().context("Failed to find the application data directory")?;
    save_dir.push("rust-snes"); // Change the directory name to "rust-snes"

    // Create the directory if it doesn't exist
    if !save_dir.exists() {
        fs::create_dir_all(&save_dir)
            .with_context(|| format!("Failed to create directory: {:?}", save_dir))?;
    }

    // Set the path for the save file
    let save_file = save_dir.join(format!("{}.srm", rom_name));

    // Write the save data
    fs::write(&save_file, sram_data)
        .with_context(|| format!("Failed to save data: {:?}", save_file))?;

    Ok(())
}

// Load save data
fn load_save_data(rom_name: &str) -> Result<Option<Vec<u8>>> {
    // Retrieve application data directory and change to "rust-snes"
    let mut save_dir = data_dir().context("Failed to find the application data directory")?;
    save_dir.push("rust-snes"); // Change the directory name to "rust-snes"

    // Set the path for the save file
    let save_file = save_dir.join(format!("{}.srm", rom_name));

    // If the save file exists, load the data
    if save_file.exists() {
        let data = fs::read(&save_file)
            .with_context(|| format!("Failed to load save data: {:?}", save_file))?;
        Ok(Some(data))
    } else {
        Ok(None)
    }
}
