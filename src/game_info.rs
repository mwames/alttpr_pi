use std::ffi::CString;

use rust_libretro_sys::retro_game_info;

pub struct GameInfo {
    rom_path: CString,
}

impl GameInfo {
    pub fn new(rom_path: CString) -> Self {
        GameInfo {
            rom_path,
        }
    }

    pub fn build(&self) -> retro_game_info {
        retro_game_info {
            path: self.rom_path.as_ptr(),
            data: std::ptr::null(),
            size: 0,
            meta: std::ptr::null(),
        }
    }
            
}