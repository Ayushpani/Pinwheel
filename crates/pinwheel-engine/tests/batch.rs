//! End-to-end batch tests on real files in a temporary folder.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use image::{DynamicImage, RgbImage};
use pinwheel_core::resize::{Dimensions, Options, Target};
use pinwheel_engine::batch::{run, Action, Settings, ZipMode};
use pinwheel_engine::image_ops::{encode, Format};
use pinwheel_engine::output::Location;

/// A noisy photo-like image, so compression behaves like it does on photos.
fn photo(w: u32, h: u32, seed: u32) -> DynamicImage {
    let mut state = seed.wrapping_mul(2_654_435_761).max(1);
    DynamicImage::ImageRgb8(RgbImage::from_fn(w, h, |x, y| {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        let n = (state % 48) as u8;
        image::Rgb([(x * 200 / w) as u8 + n, (y * 200 / h) as u8 + n, 90 + n])
    }))
}

fn write(path: &Path, format: Format, w: u32, h: u32) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, encode(&photo(w, h, w + h), format, 90).unwrap()).unwrap();
}

/// Every file under `dir`, relative, with forward slashes, sorted.
fn tree(dir: &Path) -> Vec<String> {
    fn go(base: &Path, dir: &Path, out: &mut Vec<String>) {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                go(base, &p, out);
            } else {
                let rel = p.strip_prefix(base).unwrap();
                out.push(
                    rel.components()
                        .map(|c| c.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/"),
                );
            }
        }
    }
    let mut out = Vec::new();
    go(dir, dir, &mut out);
    out.sort();
    out
}

fn zip_names(path: &Path) -> Vec<String> {
    let mut archive = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
    let mut names: Vec<String> = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().to_string())
        .collect();
    names.sort();
    names
}

/// A folder of holiday photos with a subfolder, a mix of formats and a note.
fn holiday(base: &Path) -> PathBuf {
    let root = base.join("Holiday");
    write(&root.join("beach.png"), Format::Png, 640, 480);
    write(&root.join("sunset.jpg"), Format::Jpg, 800, 600);
    write(&root.join("day2/market.bmp"), Format::Bmp, 480, 640);
    fs::write(root.join("notes.txt"), "packing list").unwrap();
    root
}

#[test]
fn bulk_convert_folder_beside_originals() {
    let tmp = tempfile::tempdir().unwrap();
    let root = holiday(tmp.path());

    let report = run(
        std::slice::from_ref(&root),
        &Action::Convert { to: Format::Webp },
        &Settings::default(),
    )
    .unwrap();

    assert_eq!(report.done.len(), 3);
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    assert_eq!(report.skipped, vec![root.join("notes.txt")]);
    assert_eq!(
        tree(&root),
        [
            "beach.png",
            "beach.webp",
            "day2/market.bmp",
            "day2/market.webp",
            "notes.txt",
            "sunset.jpg",
            "sunset.webp",
        ]
    );
    for d in &report.done {
        assert_eq!(&fs::read(&d.output).unwrap()[8..12], b"WEBP");
        assert_eq!(d.from, d.to);
    }
}

#[test]
fn bulk_resize_into_chosen_folder_keeps_structure() {
    let tmp = tempfile::tempdir().unwrap();
    let root = holiday(tmp.path());
    let out = tmp.path().join("Resized");
    let settings = Settings {
        location: Location::Folder(out.clone()),
        ..Settings::default()
    };
    let action = Action::Resize {
        target: Target::LongEdge(320),
        options: Options::default(),
        to: None,
    };

    let report = run(std::slice::from_ref(&root), &action, &settings).unwrap();

    assert_eq!(
        tree(&out),
        [
            "beach-resized.png",
            "day2/market-resized.bmp",
            "sunset-resized.jpg"
        ]
    );
    let market = report
        .done
        .iter()
        .find(|d| d.input.ends_with("market.bmp"))
        .unwrap();
    assert_eq!(market.to, Dimensions::new(240, 320));
    let back = image::open(&market.output).unwrap();
    assert_eq!((back.width(), back.height()), (240, 320));
    // The originals are untouched.
    assert_eq!(tree(&root).len(), 4);
}

#[test]
fn subfolder_output_is_not_reprocessed() {
    let tmp = tempfile::tempdir().unwrap();
    let root = holiday(tmp.path());
    let settings = Settings {
        location: Location::Subfolder("Pinwheel".into()),
        ..Settings::default()
    };
    let action = Action::Convert { to: Format::Jpg };

    run(std::slice::from_ref(&root), &action, &settings).unwrap();
    let again = run(std::slice::from_ref(&root), &action, &settings).unwrap();

    assert_eq!(again.done.len(), 3, "the Pinwheel subfolder is skipped");
    assert_eq!(
        tree(&root.join("Pinwheel")),
        [
            "beach (2).jpg",
            "beach.jpg",
            "day2/market (2).jpg",
            "day2/market.jpg",
            "sunset-converted (2).jpg",
            "sunset-converted.jpg",
        ]
    );
}

#[test]
fn zip_after_bulk_conversion() {
    let tmp = tempfile::tempdir().unwrap();
    let root = holiday(tmp.path());
    let settings = Settings {
        zip: ZipMode::Also,
        ..Settings::default()
    };

    let report = run(
        std::slice::from_ref(&root),
        &Action::Convert { to: Format::Png },
        &settings,
    )
    .unwrap();

    let zip = report.zip.unwrap();
    assert_eq!(
        zip,
        tmp.path().join("Holiday-png.zip"),
        "beside the dropped folder"
    );
    assert_eq!(
        zip_names(&zip),
        ["beach-converted.png", "day2/market.png", "sunset.png"]
    );
    assert!(root.join("sunset.png").exists(), "loose files are kept too");
}

#[test]
fn zip_only_writes_no_loose_files() {
    let tmp = tempfile::tempdir().unwrap();
    let root = holiday(tmp.path());
    let before = tree(&root);
    let settings = Settings {
        zip: ZipMode::Only,
        ..Settings::default()
    };
    let action = Action::Resize {
        target: Target::Percent(50.0),
        options: Options::default(),
        to: Some(Format::Jpg),
    };

    let report = run(std::slice::from_ref(&root), &action, &settings).unwrap();

    assert_eq!(tree(&root), before);
    let zip = report.zip.unwrap();
    assert_eq!(
        zip_names(&zip),
        ["beach.jpg", "day2/market.jpg", "sunset-resized.jpg"]
    );
    let mut archive = zip::ZipArchive::new(fs::File::open(&zip).unwrap()).unwrap();
    let mut bytes = Vec::new();
    archive
        .by_name("beach.jpg")
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    let img = image::load_from_memory(&bytes).unwrap();
    assert_eq!((img.width(), img.height()), (320, 240));
    assert_eq!(
        fs::read_dir(tmp.path()).unwrap().count(),
        2,
        "only the folder and the ZIP, no temp files"
    );
}

#[test]
fn loose_files_with_the_same_name_get_numbered() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a/photo.png");
    let b = tmp.path().join("b/photo.png");
    write(&a, Format::Png, 100, 100);
    write(&b, Format::Png, 100, 100);
    let out = tmp.path().join("out");
    let settings = Settings {
        location: Location::Folder(out.clone()),
        zip: ZipMode::Also,
        ..Settings::default()
    };

    let report = run(&[a, b], &Action::Convert { to: Format::Jpg }, &settings).unwrap();

    assert_eq!(
        tree(&out),
        ["Pinwheel-jpg.zip", "photo (2).jpg", "photo.jpg"]
    );
    assert_eq!(
        zip_names(&report.zip.unwrap()),
        ["photo (2).jpg", "photo.jpg"]
    );
}

#[test]
fn hits_a_size_target() {
    let tmp = tempfile::tempdir().unwrap();
    let big = tmp.path().join("scan.png");
    write(&big, Format::Png, 1600, 1200);
    let settings = Settings {
        max_size: Some("under 50 KB".parse().unwrap()),
        ..Settings::default()
    };

    let report = run(&[big], &Action::Convert { to: Format::Jpg }, &settings).unwrap();

    let d = &report.done[0];
    assert!(d.after <= 50_000, "{} bytes", d.after);
    assert_eq!(fs::metadata(&d.output).unwrap().len(), d.after);
}

#[test]
fn bad_files_fail_alone_with_a_plain_message() {
    let tmp = tempfile::tempdir().unwrap();
    let good = tmp.path().join("good.png");
    write(&good, Format::Png, 50, 50);
    let broken = tmp.path().join("broken.jpg");
    fs::write(&broken, b"\xFF\xD8\xFF\xE0 not really a jpeg").unwrap();
    let heic = tmp.path().join("phone.heic");
    fs::write(&heic, b"\x00\x00\x00\x18ftypheic\x00\x00\x00\x00mif1heic").unwrap();

    let report = run(
        &[good, broken.clone(), heic.clone()],
        &Action::Convert { to: Format::Webp },
        &Settings::default(),
    )
    .unwrap();

    assert_eq!(report.done.len(), 1);
    assert_eq!(report.failed.len(), 2);
    assert!(
        report.failed[0]
            .message
            .starts_with("broken.jpg could not be opened"),
        "{}",
        report.failed[0].message
    );
    assert_eq!(
        report.failed[1].message,
        "phone.heic is a HEIC image, which Pinwheel can't open yet."
    );
}

#[test]
fn png_cannot_aim_for_a_size() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("logo.png");
    write(&p, Format::Png, 50, 50);
    let settings = Settings {
        max_size: Some("under 1 KB".parse().unwrap()),
        ..Settings::default()
    };
    let report = run(&[p], &Action::Convert { to: Format::Png }, &settings).unwrap();
    assert_eq!(
        report.failed[0].message,
        "PNG files can't aim for a size. Choose JPG or WebP."
    );
}

#[test]
fn empty_folder_says_so() {
    let tmp = tempfile::tempdir().unwrap();
    let err = run(
        &[tmp.path().to_path_buf()],
        &Action::Convert { to: Format::Jpg },
        &Settings::default(),
    )
    .unwrap_err();
    assert_eq!(err, "There are no images to work on here.");
}
