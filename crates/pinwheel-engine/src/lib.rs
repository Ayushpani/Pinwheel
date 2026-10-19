//! Pinwheel engine: the part that actually makes files.
//!
//! Every shell and the command-line tool call into this crate. It reads the
//! plans from `pinwheel-core` and turns them into real output, written
//! safely and never over an existing file.

pub mod batch;
pub mod image_ops;
pub mod output;

use std::io;
use std::path::Path;

/// A plain message for a file or folder that could not be used.
pub fn open_error(path: &Path, e: &io::Error) -> String {
    match e.kind() {
        io::ErrorKind::NotFound => format!(
            "{} does not exist. Check the name and folder.",
            path.display()
        ),
        io::ErrorKind::PermissionDenied => {
            format!("{} cannot be opened: permission denied.", path.display())
        }
        _ => format!("{}: {e}", path.display()),
    }
}
