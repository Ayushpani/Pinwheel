//! Where results go, and writing them so that nothing is ever overwritten
//! and a crash never leaves a half-written file.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use pinwheel_core::naming::unique_path;
use tempfile::NamedTempFile;

/// Prefix of in-progress files. Anything left with it after a crash is
/// removed by [`clean_leftovers`].
pub const TEMP_PREFIX: &str = ".pinwheel-";

/// Where results are saved.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Location {
    /// Next to the original. The default.
    #[default]
    BesideOriginal,
    /// A subfolder, such as "Pinwheel", next to the original or inside the
    /// dropped folder.
    Subfolder(String),
    /// One fixed folder the user chose.
    Folder(PathBuf),
}

impl Location {
    /// The folder for the result of `input`. `root` is the folder the user
    /// dropped, if any, so batch output keeps its folder structure.
    pub fn dir_for(&self, input: &Path, root: Option<&Path>) -> PathBuf {
        let parent = parent_of(input);
        let inside_root = || {
            root.and_then(|r| parent.strip_prefix(r).ok())
                .map(Path::to_path_buf)
                .unwrap_or_default()
        };
        match self {
            Location::BesideOriginal => parent,
            Location::Subfolder(name) => match root {
                Some(r) => r.join(name).join(inside_root()),
                None => parent.join(name),
            },
            Location::Folder(dir) => dir.join(inside_root()),
        }
    }

    /// The folder for a ZIP of a whole batch: beside the dropped folder (or
    /// first file), in its subfolder, or in the chosen folder.
    pub fn zip_dir(&self, first_input: &Path, root: Option<&Path>) -> PathBuf {
        let base = match root {
            Some(r) => r.to_path_buf(),
            None => parent_of(first_input),
        };
        match self {
            Location::BesideOriginal if root.is_some() => match base.parent() {
                Some(p) if p.as_os_str().is_empty() => PathBuf::from("."),
                Some(p) => p.to_path_buf(),
                // The dropped folder is a drive root: nothing is beside it.
                None => base,
            },
            Location::BesideOriginal => base,
            Location::Subfolder(name) => base.join(name),
            Location::Folder(dir) => dir.clone(),
        }
    }
}

/// `Path::parent`, but "photo.jpg" gives "." rather than "".
fn parent_of(path: &Path) -> PathBuf {
    match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// Writes a new file named `file_name` in `dir` (or `name (2).ext` and so on
/// when taken). The content goes to a temporary file first and is moved into
/// place only when complete.
pub fn save_with(
    dir: &Path,
    file_name: &str,
    write: impl FnOnce(&mut fs::File) -> io::Result<()>,
) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let mut tmp = tempfile::Builder::new()
        .prefix(TEMP_PREFIX)
        .tempfile_in(dir)?;
    write(tmp.as_file_mut())?;
    tmp.as_file_mut().flush()?;
    persist_file(tmp, dir, file_name)
}

pub fn save(dir: &Path, file_name: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    save_with(dir, file_name, |f| f.write_all(bytes))
}

/// Moves a finished temporary file into place as `file_name`, numbered if taken.
pub fn persist_file(mut tmp: NamedTempFile, dir: &Path, file_name: &str) -> io::Result<PathBuf> {
    loop {
        let path = unique_path(dir, file_name, |p| p.exists());
        match tmp.persist_noclobber(&path) {
            Ok(_) => return Ok(path),
            // Another job took the name between the check and the move.
            Err(e) if e.error.kind() == io::ErrorKind::AlreadyExists => tmp = e.file,
            Err(e) => return Err(e.error),
        }
    }
}

/// Removes in-progress files a crash left behind in `dir`.
pub fn clean_leftovers(dir: &Path) -> io::Result<usize> {
    let mut removed = 0;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let is_leftover = entry.file_name().to_string_lossy().starts_with(TEMP_PREFIX);
        if is_leftover && entry.file_type()?.is_file() {
            fs::remove_file(entry.path())?;
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locations() {
        let photo = Path::new("/home/a/photos/trip/beach.jpg");
        let root = Path::new("/home/a/photos");
        assert_eq!(
            Location::BesideOriginal.dir_for(photo, Some(root)),
            Path::new("/home/a/photos/trip")
        );
        assert_eq!(
            Location::Subfolder("Pinwheel".into()).dir_for(photo, Some(root)),
            Path::new("/home/a/photos/Pinwheel/trip")
        );
        assert_eq!(
            Location::Subfolder("Pinwheel".into()).dir_for(photo, None),
            Path::new("/home/a/photos/trip/Pinwheel")
        );
        assert_eq!(
            Location::Folder("/out".into()).dir_for(photo, Some(root)),
            Path::new("/out/trip")
        );
        assert_eq!(
            Location::Folder("/out".into()).dir_for(photo, None),
            Path::new("/out")
        );
        assert_eq!(
            Location::BesideOriginal.dir_for(Path::new("beach.jpg"), None),
            Path::new(".")
        );
    }

    #[test]
    fn zip_goes_beside_the_dropped_folder() {
        let photo = Path::new("/home/a/photos/trip/beach.jpg");
        let root = Path::new("/home/a/photos");
        assert_eq!(
            Location::BesideOriginal.zip_dir(photo, Some(root)),
            Path::new("/home/a")
        );
        assert_eq!(
            Location::BesideOriginal.zip_dir(photo, None),
            Path::new("/home/a/photos/trip")
        );
        assert_eq!(
            Location::Subfolder("Pinwheel".into()).zip_dir(photo, Some(root)),
            Path::new("/home/a/photos/Pinwheel")
        );
        assert_eq!(
            Location::Folder("/out".into()).zip_dir(photo, Some(root)),
            Path::new("/out")
        );
    }

    #[test]
    fn never_overwrites_and_leaves_no_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let a = save(dir.path(), "photo.jpg", b"one").unwrap();
        let b = save(dir.path(), "photo.jpg", b"two").unwrap();
        assert_eq!(a.file_name().unwrap(), "photo.jpg");
        assert_eq!(b.file_name().unwrap(), "photo (2).jpg");
        assert_eq!(fs::read(&a).unwrap(), b"one");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn cleans_leftovers() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".pinwheel-abc123"), b"half").unwrap();
        fs::write(dir.path().join("keep.jpg"), b"x").unwrap();
        assert_eq!(clean_leftovers(dir.path()).unwrap(), 1);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
