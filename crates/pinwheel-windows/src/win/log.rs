//! A plain-text log of this session at `%LOCALAPPDATA%\Pinwheel\log.txt`.
//! It never leaves the PC; people can attach it to an issue if they want.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

static LOG: Mutex<Option<(File, Instant)>> = Mutex::new(None);

pub fn path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("Pinwheel").join("log.txt")
}

/// Starts a fresh log for this session.
pub fn start() {
    let path = path();
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(file) = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
    {
        *LOG.lock().unwrap_or_else(|e| e.into_inner()) = Some((file, Instant::now()));
    }
    write(&format!("Pinwheel {} started", env!("CARGO_PKG_VERSION")));
}

/// Adds a line, stamped with seconds since start.
pub fn write(line: &str) {
    if let Some((file, t0)) = LOG.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        let _ = writeln!(file, "{:>9.3}  {line}", t0.elapsed().as_secs_f64());
    }
}
