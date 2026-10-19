//! Bulk conversion and bulk resize: many files or whole folders in one go,
//! with an optional ZIP of the results.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use pinwheel_core::filetype::{family_of, sniff, Family};
use pinwheel_core::naming::{self, default_template, Context};
use pinwheel_core::resize::{self, Dimensions, Target};
use pinwheel_core::target::{fit_image, ImageFit, ImageLimits};
use pinwheel_core::units::{format_bytes, SizeRange};

use crate::image_ops::{self, file_name, Format};
use crate::output::{self, Location, TEMP_PREFIX};

/// Files handled in memory at once. Bounds memory on big batches.
const CHUNK: usize = 32;

/// Bytes read to tell what a file is.
const HEAD_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Convert {
        to: Format,
    },
    /// Resize, keeping each file's format unless `to` is given.
    Resize {
        target: Target,
        options: resize::Options,
        to: Option<Format>,
    },
}

impl Action {
    /// The word used in names and the ZIP: "photo-resized.jpg", "photos-webp.zip".
    fn label(&self) -> &'static str {
        match self {
            Action::Convert { to } => to.ext(),
            Action::Resize { .. } => "resized",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZipMode {
    /// Loose files only. The default.
    #[default]
    Off,
    /// Loose files and a ZIP of them.
    Also,
    /// Only the ZIP; no loose files are written.
    Only,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Settings {
    /// 1–100 for JPG and WebP. `None` uses each format's default.
    pub quality: Option<u8>,
    /// Aim for this file size by lowering quality, then dimensions.
    pub max_size: Option<SizeRange>,
    pub location: Location,
    /// Name template; `None` uses the default naming.
    pub template: Option<String>,
    pub zip: ZipMode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Done {
    pub input: PathBuf,
    /// The saved file, or its name inside the ZIP when only a ZIP was made.
    pub output: PathBuf,
    pub before: u64,
    pub after: u64,
    pub from: Dimensions,
    pub to: Dimensions,
    /// Something the user should know, such as a size target not reached.
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failed {
    pub input: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    pub done: Vec<Done>,
    pub failed: Vec<Failed>,
    /// Files in dropped folders that are not images, left alone.
    pub skipped: Vec<PathBuf>,
    pub zip: Option<PathBuf>,
}

impl Report {
    pub fn total_before(&self) -> u64 {
        self.done.iter().map(|d| d.before).sum()
    }

    pub fn total_after(&self) -> u64 {
        self.done.iter().map(|d| d.after).sum()
    }
}

/// One file to process, and the folder it was found in, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Item {
    path: PathBuf,
    root: Option<PathBuf>,
}

/// Expands dropped files and folders into the images to process. Folders are
/// walked in name order; hidden files, in-progress files and Pinwheel's own
/// output subfolder are left out.
fn collect(
    inputs: &[PathBuf],
    location: &Location,
    report: &mut Report,
) -> Result<Vec<Item>, String> {
    let skip_dir = match location {
        Location::Subfolder(name) => Some(name.as_str()),
        _ => None,
    };
    let mut items = Vec::new();
    for input in inputs {
        let meta = fs::metadata(input).map_err(|e| crate::open_error(input, &e))?;
        if meta.is_dir() {
            walk(input, input, skip_dir, &mut items, report)?;
        } else {
            items.push(Item {
                path: input.clone(),
                root: None,
            });
        }
    }
    Ok(items)
}

fn walk(
    root: &Path,
    dir: &Path,
    skip_dir: Option<&str>,
    items: &mut Vec<Item>,
    report: &mut Report,
) -> Result<(), String> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| crate::open_error(dir, &e))?
        .filter_map(Result::ok)
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name.starts_with(TEMP_PREFIX) {
            continue;
        }
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            if skip_dir != Some(name.as_str()) {
                walk(root, &path, skip_dir, items, report)?;
            }
        } else if is_image(&path) {
            items.push(Item {
                path,
                root: Some(root.to_path_buf()),
            });
        } else {
            report.skipped.push(path);
        }
    }
    Ok(())
}

fn head(path: &Path) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    File::open(path)?.take(HEAD_BYTES).read_to_end(&mut buf)?;
    Ok(buf)
}

fn is_image(path: &Path) -> bool {
    let named = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let real = head(path).ok().and_then(|h| sniff(&h));
    family_of(real.unwrap_or(named)) == Some(Family::Image)
}

/// A finished result, still in memory.
struct Encoded {
    item: Item,
    file_name: String,
    bytes: Vec<u8>,
    before: u64,
    from: Dimensions,
    to: Dimensions,
    note: Option<String>,
}

fn process(item: Item, action: &Action, settings: &Settings) -> Result<Encoded, String> {
    let path = &item.path;
    let before = fs::metadata(path)
        .map_err(|e| crate::open_error(path, &e))?
        .len();
    let source =
        image_ops::readable_format(path, &head(path).map_err(|e| crate::open_error(path, &e))?)?;
    let img = image_ops::load(path)?;
    let from = image_ops::dimensions(&img);

    let (img, to_format) = match action {
        Action::Convert { to } => (img, *to),
        Action::Resize {
            target,
            options,
            to,
        } => {
            let plan = resize::plan(from, target, options).map_err(|e| e.to_string())?;
            (image_ops::apply(&img, &plan), to.unwrap_or(source))
        }
    };

    let mut note = None;
    let (bytes, to) = match settings.max_size {
        None => {
            let q = settings.quality.unwrap_or(to_format.default_quality());
            (
                image_ops::encode(&img, to_format, q)?,
                image_ops::dimensions(&img),
            )
        }
        Some(range) => {
            if !to_format.has_quality() {
                return Err(format!(
                    "{} files can't aim for a size. Choose JPG or WebP.",
                    to_format.ext().to_ascii_uppercase()
                ));
            }
            let limits = ImageLimits {
                max_quality: settings
                    .quality
                    .unwrap_or(ImageLimits::default().max_quality),
                ..ImageLimits::default()
            };
            let mut last: Option<(Dimensions, u8, Vec<u8>)> = None;
            let mut scaled: Option<(Dimensions, image::DynamicImage)> = None;
            let fit = fit_image(image_ops::dimensions(&img), range, limits, |d, q| {
                if scaled.as_ref().is_none_or(|(sd, _)| *sd != d) {
                    scaled = Some((d, image_ops::scale(&img, d)));
                }
                let bytes = image_ops::encode(&scaled.as_ref().expect("set above").1, to_format, q)
                    .unwrap_or_default();
                let len = bytes.len() as u64;
                last = Some((d, q, bytes));
                len
            })
            .map_err(|e| format!("{}: {e}", file_name(path)))?;
            let chosen = match fit {
                ImageFit::Fits(e) => e,
                ImageFit::TooSmall(e) => {
                    note = Some(format!(
                        "Only {} at best quality, below the minimum. Enlarge it to reach the minimum.",
                        format_bytes(e.bytes)
                    ));
                    e
                }
            };
            let bytes = match last {
                Some((d, q, b)) if d == chosen.dimensions && q == chosen.quality => b,
                _ => image_ops::encode(
                    &image_ops::scale(&img, chosen.dimensions),
                    to_format,
                    chosen.quality,
                )?,
            };
            (bytes, chosen.dimensions)
        }
    };

    let stem = path
        .file_stem()
        .map_or_else(|| "image".into(), |s| s.to_string_lossy().into_owned());
    let same_format = to_format == source;
    let ctx = Context {
        name: stem,
        ext: to_format.ext().to_string(),
        action: Some(match action {
            Action::Convert { .. } => "converted".to_string(),
            Action::Resize { .. } => "resized".to_string(),
        }),
        width: Some(to.width),
        height: Some(to.height),
        ..Context::default()
    };
    let template = settings
        .template
        .as_deref()
        .unwrap_or(default_template(same_format));
    let file_name = naming::render(template, &ctx).map_err(|e| e.to_string())?;

    Ok(Encoded {
        item,
        file_name,
        bytes,
        before,
        from,
        to,
        note,
    })
}

/// Name of the batch ZIP: the dropped folder's name, the single file's name,
/// or "Pinwheel" for a mix, followed by the action.
fn zip_name(inputs: &[PathBuf], action: &Action) -> String {
    let base = match inputs {
        [one] => one
            .file_stem()
            .or_else(|| one.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Pinwheel".into()),
        _ => "Pinwheel".into(),
    };
    format!("{base}-{}.zip", action.label())
}

/// Runs `action` on every image in `inputs` (files and folders). Files that
/// fail are reported and the rest carry on.
pub fn run(inputs: &[PathBuf], action: &Action, settings: &Settings) -> Result<Report, String> {
    let mut report = Report::default();
    let items = collect(inputs, &settings.location, &mut report)?;
    if items.is_empty() {
        return Err("There are no images to work on here.".into());
    }

    let mut zip = match settings.zip {
        ZipMode::Off => None,
        ZipMode::Also | ZipMode::Only => {
            let first = &items[0];
            let dir = settings
                .location
                .zip_dir(&first.path, first.root.as_deref());
            fs::create_dir_all(&dir).map_err(|e| crate::open_error(&dir, &e))?;
            let tmp = tempfile::Builder::new()
                .prefix(TEMP_PREFIX)
                .tempfile_in(&dir)
                .map_err(|e| crate::open_error(&dir, &e))?;
            Some((dir, ZipWriter::new(tmp), HashSet::new()))
        }
    };

    for chunk in items.chunks(CHUNK) {
        let results: Vec<_> = chunk
            .par_iter()
            .map(|item| (item.path.clone(), process(item.clone(), action, settings)))
            .collect();
        for (input, result) in results {
            let enc = match result {
                Ok(enc) => enc,
                Err(message) => {
                    report.failed.push(Failed { input, message });
                    continue;
                }
            };
            let mut output = PathBuf::new();
            if settings.zip != ZipMode::Only {
                let dir = settings
                    .location
                    .dir_for(&enc.item.path, enc.item.root.as_deref());
                match output::save(&dir, &enc.file_name, &enc.bytes) {
                    Ok(path) => output = path,
                    Err(e) => {
                        report.failed.push(Failed {
                            input,
                            message: crate::open_error(&dir, &e),
                        });
                        continue;
                    }
                }
            }
            if let Some((_, writer, names)) = zip.as_mut() {
                let entry = zip_entry(&enc, names);
                let options = SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated);
                writer
                    .start_file(entry.as_str(), options)
                    .and_then(|_| writer.write_all(&enc.bytes).map_err(Into::into))
                    .map_err(|e| format!("Could not add {} to the ZIP: {e}", enc.file_name))?;
                if settings.zip == ZipMode::Only {
                    output = PathBuf::from(entry);
                }
            }
            report.done.push(Done {
                input,
                output,
                before: enc.before,
                after: enc.bytes.len() as u64,
                from: enc.from,
                to: enc.to,
                note: enc.note,
            });
        }
    }

    if let Some((dir, writer, _)) = zip {
        if report.done.is_empty() {
            return Ok(report);
        }
        let tmp = writer
            .finish()
            .map_err(|e| format!("Could not finish the ZIP: {e}"))?;
        let name = zip_name(inputs, action);
        let path =
            output::persist_file(tmp, &dir, &name).map_err(|e| crate::open_error(&dir, &e))?;
        report.zip = Some(path);
    }
    Ok(report)
}

/// The path inside the ZIP: the folder structure under the dropped folder,
/// then the file name, numbered if two results share a name.
fn zip_entry(enc: &Encoded, taken: &mut HashSet<String>) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let (Some(root), Some(parent)) = (&enc.item.root, enc.item.path.parent()) {
        if let Ok(rel) = parent.strip_prefix(root) {
            parts.extend(
                rel.components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned()),
            );
        }
    }
    let folder = if parts.is_empty() {
        String::new()
    } else {
        format!("{}/", parts.join("/"))
    };
    let unique = naming::unique_path(Path::new(""), &enc.file_name, |p| {
        taken.contains(&format!("{folder}{}", p.to_string_lossy()))
    });
    let entry = format!("{folder}{}", unique.to_string_lossy());
    taken.insert(entry.clone());
    entry
}
