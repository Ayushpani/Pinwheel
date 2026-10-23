//! Pinwheel for Windows: a tray app that shows the wheel at the cursor when
//! you press Shift while dragging files, and runs the slice you drop on.
//!
//! The modules outside `win` hold no Windows calls, so they are tested and
//! previewed on every system.

pub mod geometry;
pub mod job;
pub mod prefs;
pub mod render;
pub mod slices;

#[cfg(windows)]
pub mod win;
