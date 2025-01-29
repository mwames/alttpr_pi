use std::{f32::consts::PI, ffi::CString, fs, sync::Mutex};

use sdl3::sys::pixels::{SDL_PIXELFORMAT_ARGB8888, SDL_PIXELFORMAT_RGB565, SDL_PIXELFORMAT_XRGB8888};
use sdl3::{event::Event, rect::Rect};
use sdl3::keyboard::Keycode;
use sdl3::pixels::{Color, PixelFormat};
use sdl3::timer::ticks;
use rust_libretro_sys::{retro_deinit, retro_game_info, retro_init, retro_load_game, retro_run, retro_set_audio_sample, retro_set_audio_sample_batch, retro_set_environment, retro_set_input_poll, retro_set_input_state, retro_set_video_refresh, retro_unload_game};

use std::ffi::c_void;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 224;

/// Our implementation of the callback
unsafe extern "C" fn retro_environment(cmd: u32, data: *mut c_void) -> bool {
    match cmd {
        0 => {
            println!("retro_environment: Received cmd 0, returning false.");
            false
        }
        10 => { // RETRO_ENVIRONMENT_SET_PIXEL_FORMAT
            println!("Setting pixel format to RGB565.");
            let format = 0; // RETRO_PIXEL_FORMAT_XRGB8888
            *(data as *mut i32) = format;
            true
        }
        _ => {
            println!("retro_environment: Unknown cmd {}, returning false.", cmd);
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
    println!("retro_video_refresh: width={}, height={}, pitch={}", width, height, pitch);
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

unsafe extern "C" fn retro_input_poll() {
    // Do nothing for now, just prevent a null function pointer crash
}

unsafe extern "C" fn retro_input_state(_port: u32, _device: u32, _index: u32, _id: u32) -> i16 {
    0 // No button presses
}

unsafe extern "C" fn retro_audio_sample(_left: i16, _right: i16) {
    // Do nothing for now, just avoid null function pointer crash
}

unsafe extern "C" fn retro_audio_sample_batch(_data: *const i16, _frames: usize) -> usize {
    0 // No audio output
}

fn main() {
    let path_cstr = CString::new("/home/matt/repos/alttpr_pi/zelda.smc").expect("CString conversion failed");
    let file_game: retro_game_info = retro_game_info {
        path: path_cstr.as_ptr(),
        data: std::ptr::null(),
        size: 0,
        meta: std::ptr::null(),
    };

    unsafe {
        retro_set_environment(Some(retro_environment));
        retro_set_video_refresh(Some(retro_video_refresh));
        retro_set_input_poll(Some(retro_input_poll));
        retro_set_input_state(Some(retro_input_state));
        retro_set_audio_sample(Some(retro_audio_sample));
        retro_set_audio_sample_batch(Some(retro_audio_sample_batch));

        retro_init();
        println!("Libretro core initialized!");
        let loaded = retro_load_game(&file_game as *const retro_game_info);
        if loaded {
            println!("Game loaded!");
        } else {
            println!("Failed to load game!");
        }
    }
    // Store the keycode of the key that was pressed last
    let mut last_keycode: Option<Keycode> = None;
    // Do up the window
    let sdl_context = sdl3::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("A Link to the PI", WIDTH, HEIGHT)
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
    while last_keycode != Some(Keycode::Escape) {
        // Handle events
        let mut event_pump = sdl_context.event_pump().unwrap();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    // clean up the libretro core
                    unsafe {
                        retro_unload_game();
                        retro_deinit();
                    }
                    last_keycode = Some(Keycode::Escape);
                },
                _ => {}
            }
        }
        // Change the clear color based on the time
        // let now = ticks() as f32 / 1000.0;
        // let red: f32 = 0.5 + 0.5 * (now as f32).sin();
        // let green: f32 = 0.5 + 0.5 * ((now as f32) + PI * 2.0/3.0).sin();
        // let blue: f32 = 0.5 + 0.5 * ((now as f32) + PI * 4.0/3.0).sin();
        // let rgb = Color::RGB((red * 255.0) as u8, (green * 255.0) as u8, (blue * 255.0) as u8);
        // canvas.set_draw_color(rgb);
        // canvas.clear();
        // canvas.present();

        unsafe {
            retro_run();
        }
        let framebuffer = FRAMEBUFFER.lock().unwrap();
        let window_size = canvas.output_size().unwrap();
        println!("Window size: {:?}", window_size);
        if let Some(ref fb) = *framebuffer {
            println!("Framebuffer size: {}x{}", fb.width, fb.height);
            
            texture.update(None, &fb.data, (fb.width * 4) as usize).unwrap();
            canvas.clear();
    
            // Scale to fit window
            let window_size = canvas.output_size().unwrap();
            let dst_rect = Rect::new(0, 0, WIDTH, HEIGHT);
            
            canvas.copy(&texture, None, dst_rect).unwrap();
            canvas.present();
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}
