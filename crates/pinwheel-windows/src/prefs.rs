//! The user's choices, kept in `%APPDATA%\Pinwheel\settings.toml`.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use pinwheel_core::units::SizeRange;
use pinwheel_engine::batch::{Settings, ZipMode};
use pinwheel_engine::output::Location;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SaveTo {
    /// Next to the original. The default.
    #[default]
    Beside,
    /// A "Pinwheel" subfolder next to the original.
    Subfolder,
    /// One folder the user picked.
    Folder(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub save_to: SaveTo,
    /// Also make a ZIP when a drop has several files or a folder.
    pub zip_several: bool,
    /// The welcome note has been shown.
    pub welcomed: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            save_to: SaveTo::Beside,
            zip_several: false,
            welcomed: false,
        }
    }
}

impl Prefs {
    /// Reads the settings, falling back to the defaults if the file is
    /// missing or damaged.
    pub fn load(path: &Path) -> Prefs {
        fs::read_to_string(path)
            .ok()
            .and_then(|t| toml::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let dir = path.parent().unwrap_or(Path::new("."));
        fs::create_dir_all(dir)?;
        let text = toml::to_string(self).expect("settings always serialise");
        // Write in full first, so a crash never leaves half a settings file.
        let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
        tmp.write_all(text.as_bytes())?;
        tmp.persist(path)?;
        Ok(())
    }

    /// Engine settings for one drop.
    pub fn settings(&self, several: bool, max_size: Option<SizeRange>) -> Settings {
        Settings {
            location: match &self.save_to {
                SaveTo::Beside => Location::BesideOriginal,
                SaveTo::Subfolder => Location::Subfolder("Pinwheel".into()),
                SaveTo::Folder(dir) => Location::Folder(dir.clone()),
            },
            zip: if self.zip_several && several {
                ZipMode::Also
            } else {
                ZipMode::Off
            },
            max_size,
            ..Settings::default()
        }
    }
}

/// `%APPDATA%\Pinwheel\settings.toml`.
pub fn default_path() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("Pinwheel").join("settings.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_survives_damage() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Pinwheel/settings.toml");
        assert_eq!(Prefs::load(&path), Prefs::default());

        let prefs = Prefs {
            save_to: SaveTo::Folder(PathBuf::from("D:/Converted")),
            zip_several: true,
            welcomed: true,
        };
        prefs.save(&path).unwrap();
        prefs.save(&path).unwrap();
        assert_eq!(Prefs::load(&path), prefs);
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);

        fs::write(&path, "save_to = 42").unwrap();
        assert_eq!(Prefs::load(&path), Prefs::default());
    }

    #[test]
    fn zip_only_for_several() {
        let prefs = Prefs {
            zip_several: true,
            ..Prefs::default()
        };
        assert_eq!(prefs.settings(true, None).zip, ZipMode::Also);
        assert_eq!(prefs.settings(false, None).zip, ZipMode::Off);
    }
}
