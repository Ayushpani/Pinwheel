//! Opening, resizing and saving images.

use std::io::Cursor;
use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, RgbImage, RgbaImage};

use pinwheel_core::filetype::{canonical_ext, family_of, sniff, Family};
use pinwheel_core::resize::{auto_resampler, Dimensions, Fit, Plan, Resampler};

/// The largest size an ICO file can hold.
const ICO_MAX: u32 = 256;

/// Image formats the engine can write today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Jpg,
    Png,
    Webp,
    Gif,
    Bmp,
    Tiff,
    Ico,
}

impl Format {
    pub const ALL: [Format; 7] = [
        Format::Jpg,
        Format::Png,
        Format::Webp,
        Format::Gif,
        Format::Bmp,
        Format::Tiff,
        Format::Ico,
    ];

    pub fn from_ext(ext: &str) -> Option<Format> {
        let ext = canonical_ext(ext);
        Format::ALL.into_iter().find(|f| f.ext() == ext)
    }

    pub fn ext(self) -> &'static str {
        match self {
            Format::Jpg => "jpg",
            Format::Png => "png",
            Format::Webp => "webp",
            Format::Gif => "gif",
            Format::Bmp => "bmp",
            Format::Tiff => "tiff",
            Format::Ico => "ico",
        }
    }

    /// Whether the format has a quality setting, and so can aim for a file size.
    pub fn has_quality(self) -> bool {
        matches!(self, Format::Jpg | Format::Webp)
    }

    pub fn default_quality(self) -> u8 {
        match self {
            Format::Jpg => 85,
            _ => 80,
        }
    }

    fn keeps_transparency(self) -> bool {
        !matches!(self, Format::Jpg | Format::Bmp)
    }
}

/// The format a file really is, if the engine can open it.
pub fn readable_format(path: &Path, head: &[u8]) -> Result<Format, String> {
    let named = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let ext = sniff(head)
        .map(str::to_string)
        .unwrap_or_else(|| canonical_ext(named));
    if let Some(f) = Format::from_ext(&ext) {
        return Ok(f);
    }
    Err(match family_of(&ext) {
        Some(Family::Image) => format!(
            "{} is a {} image, which Pinwheel can't open yet.",
            file_name(path),
            ext.to_ascii_uppercase()
        ),
        _ => format!("{} is not an image.", file_name(path)),
    })
}

pub fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// Opens an image and turns it upright using the camera's rotation flag.
pub fn load(path: &Path) -> Result<DynamicImage, String> {
    let unreadable =
        |e: &dyn std::fmt::Display| format!("{} could not be opened: {e}", file_name(path));
    let mut decoder = ImageReader::open(path)
        .map_err(|e| unreadable(&e))?
        .with_guessed_format()
        .map_err(|e| unreadable(&e))?
        .into_decoder()
        .map_err(|e| unreadable(&e))?;
    let orientation = decoder.orientation().ok();
    let mut img = DynamicImage::from_decoder(decoder).map_err(|e| unreadable(&e))?;
    if let Some(o) = orientation {
        img.apply_orientation(o);
    }
    Ok(img)
}

pub fn dimensions(img: &DynamicImage) -> Dimensions {
    Dimensions::new(img.width(), img.height())
}

fn filter(src: Dimensions, dst: Dimensions) -> FilterType {
    match auto_resampler(src, dst, false) {
        Resampler::Lanczos => FilterType::Lanczos3,
        Resampler::Nearest => FilterType::Nearest,
        Resampler::Bilinear => FilterType::Triangle,
        // Catmull-Rom is the closest the image crate has to Mitchell.
        Resampler::Mitchell | Resampler::Bicubic => FilterType::CatmullRom,
    }
}

/// Scales `img` to exactly `size`.
pub fn scale(img: &DynamicImage, size: Dimensions) -> DynamicImage {
    let src = dimensions(img);
    if src == size {
        return img.clone();
    }
    img.resize_exact(size.width, size.height, filter(src, size))
}

/// Applies a resize plan: scale, then crop or pad. Fill and crop keeps the
/// centre; Pad leaves the empty space transparent (white in formats without
/// transparency).
pub fn apply(img: &DynamicImage, plan: &Plan) -> DynamicImage {
    let scaled = scale(img, plan.scaled);
    let (x, y) = plan.offset((0.5, 0.5));
    let (w, h) = (plan.output.width, plan.output.height);
    match plan.fit {
        Fit::FillCrop => scaled.crop_imm(x as u32, y as u32, w, h),
        Fit::Pad => {
            let mut canvas = RgbaImage::new(w, h);
            image::imageops::overlay(&mut canvas, &scaled.to_rgba8(), x, y);
            DynamicImage::ImageRgba8(canvas)
        }
        Fit::Inside | Fit::Stretch => scaled,
    }
}

/// Shrinks to fit within `max` on both sides, keeping the shape.
pub fn fit_within(img: &DynamicImage, max: u32) -> DynamicImage {
    if img.width() <= max && img.height() <= max {
        return img.clone();
    }
    let s = max as f64 / img.width().max(img.height()) as f64;
    let size = Dimensions::new(
        ((img.width() as f64 * s).round() as u32).max(1),
        ((img.height() as f64 * s).round() as u32).max(1),
    );
    scale(img, size)
}

/// Lays transparent pixels onto white, for formats without transparency.
fn flatten(img: &DynamicImage) -> RgbImage {
    if !img.color().has_alpha() {
        return img.to_rgb8();
    }
    let rgba = img.to_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        let blend = |c: u8| ((c as u32 * a as u32 + 255 * (255 - a as u32) + 127) / 255) as u8;
        image::Rgb([blend(r), blend(g), blend(b)])
    })
}

/// Encodes `img` as `format`. `quality` (1–100) applies to JPG and WebP.
pub fn encode(img: &DynamicImage, format: Format, quality: u8) -> Result<Vec<u8>, String> {
    let failed = |e: &dyn std::fmt::Display| {
        format!("Could not write {}: {e}", format.ext().to_ascii_uppercase())
    };
    let quality = quality.clamp(1, 100);
    let mut out = Vec::new();
    match format {
        Format::Jpg => {
            JpegEncoder::new_with_quality(&mut out, quality)
                .encode_image(&flatten(img))
                .map_err(|e| failed(&e))?;
        }
        Format::Webp => {
            let memory = if img.color().has_alpha() {
                let rgba = img.to_rgba8();
                webp::Encoder::from_rgba(&rgba, rgba.width(), rgba.height()).encode(quality as f32)
            } else {
                let rgb = img.to_rgb8();
                webp::Encoder::from_rgb(&rgb, rgb.width(), rgb.height()).encode(quality as f32)
            };
            out.extend_from_slice(&memory);
        }
        _ => {
            let img = match format {
                Format::Ico => fit_within(img, ICO_MAX),
                _ => img.clone(),
            };
            let img = if format.keeps_transparency() {
                DynamicImage::ImageRgba8(img.to_rgba8())
            } else {
                DynamicImage::ImageRgb8(flatten(&img))
            };
            let image_format = match format {
                Format::Png => ImageFormat::Png,
                Format::Gif => ImageFormat::Gif,
                Format::Bmp => ImageFormat::Bmp,
                Format::Tiff => ImageFormat::Tiff,
                Format::Ico => ImageFormat::Ico,
                Format::Jpg | Format::Webp => unreachable!(),
            };
            img.write_to(&mut Cursor::new(&mut out), image_format)
                .map_err(|e| failed(&e))?;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgb8(RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x * 255 / w) as u8, (y * 255 / h) as u8, 128])
        }))
    }

    #[test]
    fn encodes_every_format_and_reads_it_back() {
        let img = gradient(64, 48);
        for format in Format::ALL {
            let bytes = encode(&img, format, 80).unwrap();
            assert_eq!(sniff(&bytes), Some(format.ext()), "{format:?}");
            let back = image::load_from_memory(&bytes).unwrap();
            assert_eq!((back.width(), back.height()), (64, 48), "{format:?}");
        }
    }

    #[test]
    fn quality_changes_size() {
        let img = gradient(400, 300);
        for format in [Format::Jpg, Format::Webp] {
            let small = encode(&img, format, 20).unwrap().len();
            let big = encode(&img, format, 95).unwrap().len();
            assert!(small < big, "{format:?}: {small} vs {big}");
        }
    }

    #[test]
    fn flattens_onto_white() {
        let clear = DynamicImage::ImageRgba8(RgbaImage::new(2, 2));
        assert_eq!(flatten(&clear).get_pixel(0, 0).0, [255, 255, 255]);
    }

    #[test]
    fn ico_is_capped_at_256() {
        let bytes = encode(&gradient(1024, 512), Format::Ico, 80).unwrap();
        let back = image::load_from_memory(&bytes).unwrap();
        assert_eq!((back.width(), back.height()), (256, 128));
    }
}
