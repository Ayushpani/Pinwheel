// No console window behind the tray app.
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() {
    pinwheel_windows::win::run();
}

#[cfg(not(windows))]
fn main() {
    eprintln!(
        "Pinwheel for Windows runs on Windows only. On this system, use the pinwheel command."
    );
    std::process::exit(1);
}
