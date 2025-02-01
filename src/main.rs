use std::collections::HashMap;
use std::ffi::{c_void, CString};
use std::process::exit;
use std::sync::{Mutex, OnceLock};

use sdl3::sys::pixels::SDL_PIXELFORMAT_RGB565;
use sdl3::Sdl;
use sdl3::{event::Event, rect::Rect};
use sdl3::keyboard::Keycode;
use sdl3::pixels::PixelFormat;
use rust_libretro_sys::{
    retro_deinit,
    retro_game_info,
    retro_init,
    retro_load_game,
    retro_run,
    retro_set_audio_sample,
    retro_set_audio_sample_batch,
    retro_set_controller_port_device,
    retro_set_environment,
    retro_set_input_poll,
    retro_set_input_state,
    retro_set_video_refresh,
    retro_unload_game,
    RETRO_DEVICE_JOYPAD,
    RETRO_DEVICE_NONE
};

mod game_info;
use game_info::GameInfo;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 224;
const PORT: u32 = 0;

/// Our implementation of the callback
unsafe extern "C" fn retro_environment(cmd: u32, data: *mut c_void) -> bool {
    match cmd {
        _ => {
            false
        }
    }
}
#[derive(Debug, Clone)]
struct Framebuffer {
    data: Vec<u8>, // Stores pixel data
    width: u32,    // Frame width
    height: u32,   // Frame height
}
static FRAMEBUFFER: Mutex<Option<Framebuffer>> = Mutex::new(None);
unsafe extern "C" fn retro_video_refresh(data: *const c_void, width: u32, height: u32, pitch: usize) {
    if data.is_null() {
        return;
    }

    let size = (pitch * height as usize) as usize;
    let framebuffer_data = unsafe { std::slice::from_raw_parts(data as *const u8, size) };

    let mut framebuffer = FRAMEBUFFER.lock().unwrap();
    *framebuffer = Some(Framebuffer {
        data: framebuffer_data.to_vec(),
        width,
        height,
    });
}


static INPUT_STATE: OnceLock<Mutex<HashMap<(u32, u32, u32, u32), i16>>> = OnceLock::new();
fn get_input_state() -> &'static Mutex<HashMap<(u32, u32, u32, u32), i16>> {
    INPUT_STATE.get_or_init(|| Mutex::new(HashMap::new()))
}
unsafe extern "C" fn retro_input_poll() {
    // println!("retro_input_poll");
    // let input_state = get_input_state();
    // let mut input_map = input_state.lock().unwrap();
    // input_map.insert((1, 1, 0, 7), 1);
}

/*
* This function is called by the libretro core to get the state of the input
* The arguments are:
* port: The port number of the controller
* device: The device type of the controller
* index: The index of the controller
* id: The ID of the button
* The function should return 1 if the button is pressed, and 0 if it is not
*/
#[no_mangle]
unsafe extern "C" fn retro_input_state(port: u32, device: u32, index: u32, id: u32) -> i16 {
    let input_state = get_input_state();
    let input_map = input_state.lock().unwrap();
    *input_map.get(&(port, device, index, id)).unwrap_or(&0)
}

unsafe extern "C" fn retro_audio_sample(_left: i16, _right: i16) {
    // Do nothing for now, just avoid null function pointer crash
}

unsafe extern "C" fn retro_audio_sample_batch(_data: *const i16, _frames: usize) -> usize {
    0 // No audio output
}

// ===== BUNCH OF GARBAGE =====
fn get_joystick(sdl_context:  &Sdl) -> sdl3::joystick::Joystick {
    let joystick_subsystem = sdl_context.joystick().unwrap();
    joystick_subsystem.set_joystick_events_enabled(true);
    let mut joysticks = joystick_subsystem.joysticks().unwrap();
    let js = joysticks.pop().unwrap();
    joystick_subsystem.open(js).unwrap()
}

// TODO: Consider using a hashmap to map SDL keycodes to SNES button IDs
fn sdl_to_snes(sdl_keycode: u8) -> u32 {
    match sdl_keycode {
        0 => 0, // B
        1 => 8, // A
        2 => 1, // Y
        3 => 9, // X
        4 => 10, // L
        5 => 11, // R
        6 => 2, // SELECT
        7 => 3, // START
        _ => sdl_keycode as u32,
    }
}

fn handle_input(sdl_context: &Sdl) {
    // Handle events
    let input_state = get_input_state();
    let mut input_map = input_state.lock().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                // clean up the libretro core
                unsafe {
                    retro_unload_game();
                    retro_deinit();
                }
                exit(0);
            },
            Event::JoyButtonDown { button_idx, .. } => {
                if button_idx < 8 {
                    input_map.insert((PORT, 1, 0, sdl_to_snes(button_idx) as u32), 1);
                }
            },
            Event::JoyButtonUp { button_idx, .. } => {
                if button_idx < 8 {
                    input_map.insert((PORT, 1, 0, sdl_to_snes(button_idx) as u32), 0);
                }
            },
            Event::JoyHatMotion { state, .. } => {
                match state {
                    // No Input
                    sdl3::joystick::HatState::Centered => {
                        input_map.insert((PORT, 1, 0, 4), 0);
                        input_map.insert((PORT, 1, 0, 5), 0);
                        input_map.insert((PORT, 1, 0, 6), 0);
                        input_map.insert((PORT, 1, 0, 7), 0);
                    },
                    // Cardinal Directions
                    sdl3::joystick::HatState::Up => {
                        input_map.insert((PORT, 1, 0, 4), 1);
                        input_map.insert((PORT, 1, 0, 5), 0);
                        input_map.insert((PORT, 1, 0, 6), 0);
                        input_map.insert((PORT, 1, 0, 7), 0);
                    },
                    sdl3::joystick::HatState::Down => {
                        input_map.insert((PORT, 1, 0, 4), 0);
                        input_map.insert((PORT, 1, 0, 5), 1);
                        input_map.insert((PORT, 1, 0, 6), 0);
                        input_map.insert((PORT, 1, 0, 7), 0);
                    },
                    sdl3::joystick::HatState::Left => {
                        input_map.insert((PORT, 1, 0, 4), 0);
                        input_map.insert((PORT, 1, 0, 5), 0);
                        input_map.insert((PORT, 1, 0, 6), 1);
                        input_map.insert((PORT, 1, 0, 7), 0);
                    },
                    sdl3::joystick::HatState::Right => {
                        input_map.insert((PORT, 1, 0, 4), 0);
                        input_map.insert((PORT, 1, 0, 5), 0);
                        input_map.insert((PORT, 1, 0, 6), 0);
                        input_map.insert((PORT, 1, 0, 7), 1);
                    },
                    // Diagonal Directions
                    sdl3::joystick::HatState::RightUp => {
                        input_map.insert((PORT, 1, 0, 4), 1);
                        input_map.insert((PORT, 1, 0, 5), 0);
                        input_map.insert((PORT, 1, 0, 6), 0);
                        input_map.insert((PORT, 1, 0, 7), 1);
                    },
                    sdl3::joystick::HatState::RightDown => {
                        input_map.insert((PORT, 1, 0, 4), 0);
                        input_map.insert((PORT, 1, 0, 5), 1);
                        input_map.insert((PORT, 1, 0, 6), 0);
                        input_map.insert((PORT, 1, 0, 7), 1);
                    },
                    sdl3::joystick::HatState::LeftUp => {
                        input_map.insert((PORT, 1, 0, 4), 1);
                        input_map.insert((PORT, 1, 0, 5), 0);
                        input_map.insert((PORT, 1, 0, 6), 1);
                        input_map.insert((PORT, 1, 0, 7), 0);
                    },
                    sdl3::joystick::HatState::LeftDown => {
                        input_map.insert((PORT, 1, 0, 4), 0);
                        input_map.insert((PORT, 1, 0, 5), 1);
                        input_map.insert((PORT, 1, 0, 6), 1);
                        input_map.insert((PORT, 1, 0, 7), 0);
                    }
                }
            },
            _ => {}
        }
    }
    // println!("poop: {:?}", input_map);
}

fn main() {
    let game_info = GameInfo::new(CString::new("/home/matt/repos/alttpr_pi/rand.sfc").unwrap());

    let loaded = unsafe {
        retro_set_environment(Some(retro_environment));
        retro_set_video_refresh(Some(retro_video_refresh));
        retro_set_input_poll(Some(retro_input_poll));
        retro_set_input_state(Some(retro_input_state));
        retro_set_audio_sample(Some(retro_audio_sample));
        retro_set_audio_sample_batch(Some(retro_audio_sample_batch));

        retro_init();
        println!("Libretro core initialized!");
        retro_load_game(&game_info.build() as *const retro_game_info)
    };
    
    if loaded {
        println!("Game loaded!");
    } else {
        println!("Failed to load game!");
    }

    // Set up controllers
    unsafe {
        retro_set_controller_port_device(0, RETRO_DEVICE_JOYPAD);
        retro_set_controller_port_device(1, RETRO_DEVICE_NONE);
    
    }

    // Store the keycode of the key that was pressed last
    let mut last_keycode: Option<Keycode> = None;
    // Do up the window
    let sdl_context = sdl3::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("A Link to the PI", WIDTH * 4, HEIGHT * 4)
        .position_centered()
        .resizable()
        .build()
        .unwrap();
    // Create a canvas to draw on
    let mut canvas = window.into_canvas();
    let texture_creator = canvas.texture_creator();
    let pixel_format = PixelFormat::try_from(SDL_PIXELFORMAT_RGB565).unwrap();
    let mut texture = texture_creator
        .create_texture_streaming(pixel_format, WIDTH, HEIGHT)
        .expect("Failed to create texture");
    // Run while the last key pressed was not the escape key

    let joystick = get_joystick(&sdl_context);

    loop {
        handle_input(&sdl_context);
        unsafe {
            retro_run();
        }
        let framebuffer = FRAMEBUFFER.lock().unwrap();
        if let Some(ref fb) = *framebuffer {
            texture.update(None, &fb.data, (fb.width * 4) as usize).unwrap();
            canvas.clear();
    
            // Scale to fit window
            let window_size = canvas.output_size().unwrap();
            let dst_rect = Rect::new(0, 0, window_size.0, window_size.1);
            
            canvas.copy(&texture, None, dst_rect).unwrap();
            canvas.present();
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}
