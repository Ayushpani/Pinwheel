//! Exact-size targets: hitting "under X" or "between X and Y" for images,
//! PDFs, video and audio.

use crate::resize::Dimensions;
use crate::units::{format_bytes, SizeRange};
use crate::Error;

/// Image resolutions tried in turn when re-compressing the images inside a PDF.
pub const PDF_DPI_STEPS: [u32; 4] = [300, 200, 150, 100];

/// Standard audio bitrates in kbps, lowest first.
pub const AUDIO_BITRATES: [u32; 17] = [
    8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
];

/// Share of a video target kept back for the container.
const CONTAINER_RESERVE: f64 = 0.03;

/// Lowest video bitrate worth encoding; below this the picture falls apart.
const MIN_VIDEO_KBPS: f64 = 100.0;

fn kilobits(bytes: u64) -> f64 {
    bytes as f64 * 8.0 / 1000.0
}

/// video kbps = 0.97 × target size in kilobits / duration in seconds − audio kbps
pub fn video_kbps(target_bytes: u64, duration_secs: f64, audio_kbps: u32) -> Result<u32, Error> {
    if !(duration_secs.is_finite() && duration_secs > 0.0) {
        return Err(Error::CannotFit(
            "The video has no length to work from.".into(),
        ));
    }
    let total = (1.0 - CONTAINER_RESERVE) * kilobits(target_bytes) / duration_secs;
    let video = total - audio_kbps as f64;
    if video < MIN_VIDEO_KBPS {
        return Err(Error::CannotFit(format!(
            "{} is too small for {:.0} seconds of video. Trim the video, or allow at least {}.",
            format_bytes(target_bytes),
            duration_secs,
            format_bytes(min_video_bytes(duration_secs, audio_kbps)),
        )));
    }
    Ok(video.floor() as u32)
}

fn min_video_bytes(duration_secs: f64, audio_kbps: u32) -> u64 {
    let kbits = (MIN_VIDEO_KBPS + audio_kbps as f64) * duration_secs / (1.0 - CONTAINER_RESERVE);
    (kbits * 1000.0 / 8.0).ceil() as u64
}

/// Bitrate from size and duration, rounded down to the nearest standard bitrate.
pub fn audio_kbps(target_bytes: u64, duration_secs: f64) -> Result<u32, Error> {
    if !(duration_secs.is_finite() && duration_secs > 0.0) {
        return Err(Error::CannotFit(
            "The audio has no length to work from.".into(),
        ));
    }
    let ceiling = kilobits(target_bytes) / duration_secs;
    AUDIO_BITRATES
        .iter()
        .rev()
        .copied()
        .find(|&b| b as f64 <= ceiling)
        .ok_or_else(|| {
            Error::CannotFit(format!(
                "{} is too small for {:.0} seconds of audio. Trim it, or allow a bigger file.",
                format_bytes(target_bytes),
                duration_secs
            ))
        })
}

/// Limits for the image search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageLimits {
    pub min_quality: u8,
    pub max_quality: u8,
    /// The spec's minimum size; dimensions never shrink below it.
    pub min_dimensions: Option<Dimensions>,
}

impl Default for ImageLimits {
    fn default() -> Self {
        ImageLimits {
            min_quality: 30,
            max_quality: 95,
            min_dimensions: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Encoded {
    pub dimensions: Dimensions,
    pub quality: u8,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageFit {
    /// Lands inside the range.
    Fits(Encoded),
    /// Even the best quality is under the minimum. Pinwheel never pads files
    /// with junk; the caller offers to enlarge the dimensions instead.
    TooSmall(Encoded),
}

/// Finds the highest quality that fits. If the lowest allowed quality is
/// still too big, shrinks the dimensions 5% at a time, never below the
/// spec's minimum. `encode` returns the encoded size in bytes and must grow
/// with quality.
pub fn fit_image(
    src: Dimensions,
    range: SizeRange,
    limits: ImageLimits,
    mut encode: impl FnMut(Dimensions, u8) -> u64,
) -> Result<ImageFit, Error> {
    let max = range.max.unwrap_or(u64::MAX);
    let mut dims = src;
    loop {
        if let Some(best) = highest_quality_under(dims, max, limits, &mut encode) {
            return Ok(if range.min.is_some_and(|m| best.bytes < m) {
                ImageFit::TooSmall(best)
            } else {
                ImageFit::Fits(best)
            });
        }
        let next = Dimensions::new(
            ((dims.width as f64) * 0.95).floor().max(1.0) as u32,
            ((dims.height as f64) * 0.95).floor().max(1.0) as u32,
        );
        let below_min = limits
            .min_dimensions
            .is_some_and(|m| next.width < m.width || next.height < m.height);
        if below_min || next == dims {
            let smallest = encode(dims, limits.min_quality);
            return Err(Error::CannotFit(format!(
                "This image cannot get under {}. The smallest it can be at {} px is {}.",
                format_bytes(max),
                dims,
                format_bytes(smallest)
            )));
        }
        dims = next;
    }
}

fn highest_quality_under(
    dims: Dimensions,
    max: u64,
    limits: ImageLimits,
    encode: &mut impl FnMut(Dimensions, u8) -> u64,
) -> Option<Encoded> {
    let (mut lo, mut hi) = (limits.min_quality, limits.max_quality);
    let mut best = None;
    while lo <= hi {
        let q = lo + (hi - lo) / 2;
        let bytes = encode(dims, q);
        if bytes <= max {
            best = Some(Encoded {
                dimensions: dims,
                quality: q,
                bytes,
            });
            if q == u8::MAX {
                break;
            }
            lo = q + 1;
        } else if q == 0 {
            break;
        } else {
            hi = q - 1;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_formula() {
        // 25 MB for a 120 s clip with 128 kbps audio:
        // 0.97 × 200,000 kbit / 120 s − 128 = 1488.67 kbps
        assert_eq!(video_kbps(25_000_000, 120.0, 128).unwrap(), 1488);
        assert!(video_kbps(100_000, 600.0, 128).is_err());
        assert!(video_kbps(25_000_000, 0.0, 128).is_err());
    }

    #[test]
    fn audio_rounds_down_to_standard() {
        // 5 MB over 4 minutes: 166.7 kbps → 160
        assert_eq!(audio_kbps(5_000_000, 240.0).unwrap(), 160);
        assert_eq!(audio_kbps(100_000_000, 60.0).unwrap(), 320);
        assert!(audio_kbps(1_000, 600.0).is_err());
    }

    /// A fake encoder: bytes grow with pixels and quality.
    fn fake(d: Dimensions, q: u8) -> u64 {
        (d.width as u64 * d.height as u64 * q as u64) / 400
    }

    #[test]
    fn picks_highest_quality_that_fits() {
        let src = Dimensions::new(400, 300);
        let range: SizeRange = "20-25 KB".parse().unwrap();
        let ImageFit::Fits(e) = fit_image(src, range, ImageLimits::default(), fake).unwrap() else {
            panic!("should fit");
        };
        assert_eq!(e.dimensions, src);
        assert_eq!(e.quality, 83);
        assert!(range.contains(e.bytes));
    }

    #[test]
    fn shrinks_when_lowest_quality_is_too_big() {
        let src = Dimensions::new(2000, 1500);
        let range: SizeRange = "under 50 KB".parse().unwrap();
        let ImageFit::Fits(e) = fit_image(src, range, ImageLimits::default(), fake).unwrap() else {
            panic!("should fit");
        };
        assert!(e.dimensions.width < 2000);
        assert!(e.bytes <= 50_000);
    }

    #[test]
    fn says_so_when_it_cannot_fit() {
        let src = Dimensions::new(2000, 1500);
        let limits = ImageLimits {
            min_dimensions: Some(Dimensions::new(1900, 1400)),
            ..ImageLimits::default()
        };
        let err = fit_image(src, "under 50 KB".parse().unwrap(), limits, fake).unwrap_err();
        assert!(err.to_string().contains("cannot get under 50 KB"), "{err}");
    }

    #[test]
    fn reports_too_small_instead_of_padding() {
        let src = Dimensions::new(100, 100);
        let r = fit_image(
            src,
            "20-50 KB".parse().unwrap(),
            ImageLimits::default(),
            fake,
        )
        .unwrap();
        assert!(matches!(r, ImageFit::TooSmall(e) if e.quality == 95));
    }
}
