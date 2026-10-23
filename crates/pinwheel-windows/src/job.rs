//! What the hub says about a drop, and running a drop on a slice.

use std::fs;
use std::path::{Path, PathBuf};

use pinwheel_core::filetype::{family_of, Family};
use pinwheel_core::units::{describe_change, format_bytes};
use pinwheel_engine::batch;
use pinwheel_engine::image_ops::file_name;

use crate::prefs::Prefs;
use crate::slices::Slice;

/// The two hub lines for dragged files, and whether the image ring applies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub title: String,
    pub detail: String,
    pub usable: bool,
}

fn is_image(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    family_of(ext) == Some(Family::Image)
}

pub fn summarise(paths: &[PathBuf]) -> Summary {
    let folders = paths.iter().filter(|p| p.is_dir()).count();
    let images = paths.iter().filter(|p| is_image(p)).count();
    let usable = folders > 0 || images > 0;
    match paths {
        [] => Summary {
            title: "Nothing to drop".into(),
            detail: String::new(),
            usable: false,
        },
        [one] if folders == 1 => Summary {
            title: file_name(one),
            detail: "Folder".into(),
            usable,
        },
        [one] => {
            let bytes = fs::metadata(one).map(|m| m.len()).unwrap_or(0);
            let mut detail = format_bytes(bytes);
            if let Ok(size) = imagesize::size(one) {
                detail.push_str(&format!(" · {} × {}", size.width, size.height));
            }
            if !usable {
                detail = "Only images for now".into();
            }
            Summary {
                title: file_name(one),
                detail,
                usable,
            }
        }
        many => {
            let files = many.len() - folders;
            let count = |n: usize, word: &str| {
                if n == 1 {
                    format!("1 {word}")
                } else {
                    format!("{n} {word}s")
                }
            };
            let title = match (files, folders) {
                (f, 0) => count(f, "file"),
                (0, d) => count(d, "folder"),
                (f, d) => format!("{}, {}", count(f, "file"), count(d, "folder")),
            };
            let detail = if usable {
                match files - images {
                    0 => "Drop on a slice".into(),
                    1 => "1 is not an image, left alone".into(),
                    n => format!("{n} are not images, left alone"),
                }
            } else {
                "Only images for now".into()
            };
            Summary {
                title,
                detail,
                usable,
            }
        }
    }
}

/// The result notification, and the file to show when it is clicked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub title: String,
    pub message: String,
    pub reveal: Option<PathBuf>,
    pub ok: bool,
}

/// Runs `slice` on the dropped paths. Blocking: call it off the UI thread.
pub fn run(paths: &[PathBuf], slice: &Slice, prefs: &Prefs) -> Outcome {
    let (action, max_size) = slice.job();
    let several = paths.len() > 1 || paths.iter().any(|p| p.is_dir());
    let settings = prefs.settings(several, max_size);
    let report = match batch::run(paths, &action, &settings) {
        Ok(r) => r,
        Err(message) => {
            return Outcome {
                title: "Nothing to do".into(),
                message,
                reveal: None,
                ok: false,
            };
        }
    };

    let done = report.done.len();
    let failed = report.failed.len();
    let mut lines = Vec::new();
    match report.done.as_slice() {
        [] => {}
        [one] => lines.push(format!(
            "{} → {} · {}",
            file_name(&one.input),
            file_name(&one.output),
            describe_change(one.before, one.after)
        )),
        _ => lines.push(format!(
            "{done} files · {}",
            describe_change(report.total_before(), report.total_after())
        )),
    }
    if let Some(note) = report.done.iter().find_map(|d| d.note.as_ref()) {
        lines.push(note.clone());
    }
    if let Some(zip) = &report.zip {
        lines.push(format!("ZIP: {}", file_name(zip)));
    }
    if let Some(f) = report.failed.first() {
        lines.push(f.message.clone());
        if failed > 1 {
            lines.push(format!("and {} more could not be done", failed - 1));
        }
    }

    let title = match (done, failed) {
        (_, 0) => slice.done.to_string(),
        (0, _) => "Nothing was done".to_string(),
        (d, f) => format!("{d} of {} done", d + f),
    };
    let reveal = report
        .zip
        .clone()
        .or_else(|| report.done.first().map(|d| d.output.clone()));
    Outcome {
        title,
        message: lines.join("\n"),
        reveal,
        ok: failed == 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slices::image_ring;
    use pinwheel_engine::image_ops::{encode, Format};

    fn png(path: &Path, w: u32, h: u32) {
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([x as u8, y as u8, 99])
        }));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, encode(&img, Format::Png, 90).unwrap()).unwrap();
    }

    #[test]
    fn hub_lines() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("beach.png");
        png(&a, 40, 30);
        let s = summarise(std::slice::from_ref(&a));
        assert_eq!(s.title, "beach.png");
        assert!(s.detail.ends_with("· 40 × 30"), "{}", s.detail);
        assert!(s.usable);

        let notes = dir.path().join("notes.txt");
        fs::write(&notes, "x").unwrap();
        assert_eq!(
            summarise(std::slice::from_ref(&notes)).detail,
            "Only images for now"
        );
        assert!(!summarise(std::slice::from_ref(&notes)).usable);

        let s = summarise(&[a.clone(), notes.clone(), dir.path().to_path_buf()]);
        assert_eq!(s.title, "2 files, 1 folder");
        assert_eq!(s.detail, "1 is not an image, left alone");
    }

    #[test]
    fn runs_a_drop() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("Holiday/beach.png");
        let b = dir.path().join("Holiday/sunset.png");
        png(&a, 64, 48);
        png(&b, 64, 48);
        let webp = &image_ring()[2];

        let one = run(std::slice::from_ref(&a), webp, &Prefs::default());
        assert!(one.ok);
        assert_eq!(one.title, "Converted to WebP");
        assert!(
            one.message.starts_with("beach.png → beach.webp · "),
            "{}",
            one.message
        );
        assert_eq!(one.reveal, Some(dir.path().join("Holiday/beach.webp")));

        let prefs = Prefs {
            zip_several: true,
            ..Prefs::default()
        };
        let folder = run(&[dir.path().join("Holiday")], &image_ring()[0], &prefs);
        assert!(
            folder.message.contains("ZIP: Holiday-jpg.zip"),
            "{}",
            folder.message
        );
        assert_eq!(folder.reveal, Some(dir.path().join("Holiday-jpg.zip")));
    }

    #[test]
    fn failures_are_plain() {
        let dir = tempfile::tempdir().unwrap();
        let fake = dir.path().join("phone.heic");
        fs::write(&fake, b"\x00\x00\x00\x18ftypheic\x00\x00\x00\x00mif1heic").unwrap();
        let out = run(&[fake], &image_ring()[0], &Prefs::default());
        assert!(!out.ok);
        assert_eq!(out.title, "Nothing was done");
        assert_eq!(
            out.message,
            "phone.heic is a HEIC image, which Pinwheel can't open yet."
        );
    }
}
