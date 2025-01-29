fn main() {
    println!("cargo:rustc-link-search=native=lib");
    println!("cargo:rustc-link-lib=dylib=snes9x_libretro");
    println!("cargo:rustc-link-lib=dylib=SDL3");
}