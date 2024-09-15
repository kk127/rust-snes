extern crate sdl2;

use sdl2::event::Event;
use sdl2::GameControllerSubsystem;

fn main() -> Result<(), String> {
    // SDL2の初期化
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let _window = video_subsystem
        .window("SDL2 Controller Test", 800, 600)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    // GameControllerのサブシステムを取得
    let game_controller_subsystem = sdl_context.game_controller()?;

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

    // イベントポンプを作成
    let mut event_pump = sdl_context.event_pump()?;

    // イベントループ
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::ControllerButtonDown { button, .. } => {
                    println!("Button pressed: {:?}", button);
                }
                Event::ControllerButtonUp { button, .. } => {
                    println!("Button released: {:?}", button);
                }
                // Event::ControllerAxisMotion { axis, value, .. } => {
                //     println!("Axis {:?} moved to {}", axis, value);
                // }
                _ => {}
            }
        }
    }

    Ok(())
}
