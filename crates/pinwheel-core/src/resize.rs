//! Resize planning: from a source size and one of the six ways to set a size,
//! work out exactly what pixels come out, before any pixel is touched.

use std::fmt;
use std::str::FromStr;

use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

impl Dimensions {
    pub const fn new(width: u32, height: u32) -> Self {
        Dimensions { width, height }
    }

    pub fn long_edge(&self) -> u32 {
        self.width.max(self.height)
    }

    pub fn short_edge(&self) -> u32 {
        self.width.min(self.height)
    }

    pub fn aspect(&self) -> f64 {
        self.width as f64 / self.height as f64
    }

    pub fn megapixels(&self) -> f64 {
        self.width as f64 * self.height as f64 / 1e6
    }
}

impl fmt::Display for Dimensions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} × {}", self.width, self.height)
    }
}

impl FromStr for Dimensions {
    type Err = Error;

    /// Accepts "1920x1080", "1920 × 1080" and "1920*1080".
    fn from_str(s: &str) -> Result<Self, Error> {
        let bad = || Error::InvalidDimensions(s.into());
        let (w, h) = s.split_once(['x', 'X', '×', '*']).ok_or_else(bad)?;
        let width: u32 = w.trim().parse().map_err(|_| bad())?;
        let height: u32 = h.trim().parse().map_err(|_| bad())?;
        if width == 0 || height == 0 {
            return Err(bad());
        }
        Ok(Dimensions { width, height })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthUnit {
    Mm,
    Cm,
    In,
}

/// A print length, such as the 35 mm width of a passport photo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Length {
    pub value: f64,
    pub unit: LengthUnit,
}

impl Length {
    pub fn millimetres(&self) -> f64 {
        match self.unit {
            LengthUnit::Mm => self.value,
            LengthUnit::Cm => self.value * 10.0,
            LengthUnit::In => self.value * 25.4,
        }
    }

    /// pixels = millimetres / 25.4 × DPI, rounded to the nearest pixel.
    pub fn to_pixels(&self, dpi: f64) -> u32 {
        (self.millimetres() / 25.4 * dpi).round().max(1.0) as u32
    }
}

impl FromStr for Length {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        let t = s.trim();
        let split = t
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(t.len());
        let (num, unit) = t.split_at(split);
        let value: f64 = num.parse().map_err(|_| Error::InvalidLength(s.into()))?;
        let unit = match unit.trim().to_ascii_lowercase().as_str() {
            "mm" => LengthUnit::Mm,
            "cm" => LengthUnit::Cm,
            "in" | "inch" | "inches" | "\"" => LengthUnit::In,
            _ => return Err(Error::InvalidLength(s.into())),
        };
        if !(value.is_finite() && value > 0.0) {
            return Err(Error::InvalidLength(s.into()));
        }
        Ok(Length { value, unit })
    }
}

/// How an image meets a width × height box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fit {
    /// The whole image fits within the box; one side may come out smaller.
    #[default]
    Inside,
    /// The box is filled and the overflow is cut.
    FillCrop,
    /// The whole image fits and the rest of the box is filled.
    Pad,
    /// Exact size, shape distorted.
    Stretch,
}

/// The six ways to set a size, minus "file size", which lives in [`crate::target`].
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    Percent(f64),
    LongEdge(u32),
    ShortEdge(u32),
    Box {
        size: Dimensions,
        fit: Fit,
    },
    Print {
        width: Length,
        height: Length,
        dpi: f64,
        fit: Fit,
    },
    Megapixels(f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    pub never_enlarge: bool,
    /// Round output dimensions to a multiple of this (1, 2, 4, 8 or 16).
    pub round_to: u32,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            never_enlarge: true,
            round_to: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Warning {
    /// Enlarging by more than 200%.
    Enlarging { percent: u32 },
    /// Stretch changes the shape by more than 2%.
    Distorted { percent: u32 },
    /// Never enlarge kept the image at its original size.
    KeptOriginalSize,
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Warning::Enlarging { percent } => write!(
                f,
                "Enlarging to {percent}% adds blur, not detail."
            ),
            Warning::Distorted { percent } => write!(
                f,
                "Stretching changes the shape by {percent}%. Choose Fill and crop or Pad to keep it."
            ),
            Warning::KeptOriginalSize => write!(
                f,
                "Kept the original size because it is already smaller. Turn off Never enlarge to make it bigger."
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    /// Size the image is scaled to, before any crop or padding.
    pub scaled: Dimensions,
    /// Size of the output file.
    pub output: Dimensions,
    pub fit: Fit,
    pub warnings: Vec<Warning>,
}

impl Plan {
    /// Top-left corner of the area kept from `scaled` (Fill and crop) or of
    /// the image on the canvas (Pad). `focal` is where the user dragged the
    /// focal point, from (0, 0) at the top left to (1, 1) at the bottom right.
    pub fn offset(&self, focal: (f64, f64)) -> (i64, i64) {
        let axis = |scaled: u32, out: u32, f: f64| -> i64 {
            let (s, o) = (scaled as f64, out as f64);
            match self.fit {
                Fit::FillCrop => (f.clamp(0.0, 1.0) * s - o / 2.0)
                    .clamp(0.0, (s - o).max(0.0))
                    .round() as i64,
                Fit::Pad => ((o - s) / 2.0).floor() as i64,
                Fit::Inside | Fit::Stretch => 0,
            }
        };
        (
            axis(self.scaled.width, self.output.width, focal.0),
            axis(self.scaled.height, self.output.height, focal.1),
        )
    }
}

fn check_range(what: &'static str, got: f64, min: f64, max: f64) -> Result<(), Error> {
    if got.is_finite() && (min..=max).contains(&got) {
        Ok(())
    } else {
        Err(Error::OutOfRange {
            what,
            min,
            max,
            got,
        })
    }
}

fn scale(px: u32, factor: f64) -> u32 {
    (px as f64 * factor).round().max(1.0) as u32
}

fn round_to(px: u32, n: u32, down: bool) -> u32 {
    if n <= 1 {
        return px;
    }
    let r = if down {
        px / n * n
    } else {
        (px + n / 2) / n * n
    };
    r.max(n)
}

/// Works out the output size for `src` resized to `target`.
pub fn plan(src: Dimensions, target: &Target, opts: &Options) -> Result<Plan, Error> {
    if src.width == 0 || src.height == 0 {
        return Err(Error::InvalidDimensions(src.to_string()));
    }
    let (w, h) = (src.width as f64, src.height as f64);

    let (mut sx, mut sy, fit, boxed) = match *target {
        Target::Percent(p) => {
            check_range("Percentage", p, 1.0, 400.0)?;
            (p / 100.0, p / 100.0, Fit::Inside, None)
        }
        Target::LongEdge(n) => {
            check_range("Long edge", n as f64, 1.0, 65_535.0)?;
            let s = n as f64 / src.long_edge() as f64;
            (s, s, Fit::Inside, None)
        }
        Target::ShortEdge(n) => {
            check_range("Short edge", n as f64, 1.0, 65_535.0)?;
            let s = n as f64 / src.short_edge() as f64;
            (s, s, Fit::Inside, None)
        }
        Target::Megapixels(mp) => {
            check_range("Megapixels", mp, 0.01, 1_000.0)?;
            let s = (mp * 1e6 / (w * h)).sqrt();
            (s, s, Fit::Inside, None)
        }
        Target::Box { size, fit } => box_scale(src, size, fit),
        Target::Print {
            width,
            height,
            dpi,
            fit,
        } => {
            check_range("DPI", dpi, 1.0, 10_000.0)?;
            let size = Dimensions::new(width.to_pixels(dpi), height.to_pixels(dpi));
            box_scale(src, size, fit)
        }
    };

    let mut warnings = Vec::new();
    if opts.never_enlarge && (sx > 1.0 || sy > 1.0) {
        sx = sx.min(1.0);
        sy = sy.min(1.0);
        warnings.push(Warning::KeptOriginalSize);
    }

    let mut scaled = Dimensions::new(scale(src.width, sx), scale(src.height, sy));
    let output = match (fit, boxed) {
        (Fit::FillCrop, Some(b)) => Dimensions::new(
            round_to(b.width.min(scaled.width), opts.round_to, true),
            round_to(b.height.min(scaled.height), opts.round_to, true),
        ),
        (Fit::Pad, Some(b)) => Dimensions::new(
            round_to(b.width, opts.round_to, false),
            round_to(b.height, opts.round_to, false),
        ),
        _ => {
            scaled = Dimensions::new(
                round_to(scaled.width, opts.round_to, false),
                round_to(scaled.height, opts.round_to, false),
            );
            scaled
        }
    };

    let growth = sx.max(sy);
    if growth > 2.0 {
        warnings.push(Warning::Enlarging {
            percent: (growth * 100.0).round() as u32,
        });
    }
    if fit == Fit::Stretch {
        let change = (scaled.aspect() / src.aspect() - 1.0).abs();
        if change > 0.02 {
            warnings.push(Warning::Distorted {
                percent: (change * 100.0).round() as u32,
            });
        }
    }

    Ok(Plan {
        scaled,
        output,
        fit,
        warnings,
    })
}

fn box_scale(src: Dimensions, b: Dimensions, fit: Fit) -> (f64, f64, Fit, Option<Dimensions>) {
    let fx = b.width as f64 / src.width as f64;
    let fy = b.height as f64 / src.height as f64;
    match fit {
        Fit::Inside | Fit::Pad => (fx.min(fy), fx.min(fy), fit, Some(b)),
        Fit::FillCrop => (fx.max(fy), fx.max(fy), fit, Some(b)),
        Fit::Stretch => (fx, fy, fit, Some(b)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resampler {
    Lanczos,
    Bicubic,
    Mitchell,
    Bilinear,
    Nearest,
}

/// The "Auto" resampler: Lanczos to shrink, Mitchell to enlarge, Nearest for
/// pixel art at whole-number scales.
pub fn auto_resampler(src: Dimensions, dst: Dimensions, pixel_art: bool) -> Resampler {
    let whole = |a: u32, b: u32| a.max(b) % a.min(b) == 0;
    let same_factor =
        (dst.width as u64 * src.height as u64) == (dst.height as u64 * src.width as u64);
    if pixel_art && same_factor && whole(src.width, dst.width) && whole(src.height, dst.height) {
        return Resampler::Nearest;
    }
    if (dst.width as u64 * dst.height as u64) > (src.width as u64 * src.height as u64) {
        Resampler::Mitchell
    } else {
        Resampler::Lanczos
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PHOTO: Dimensions = Dimensions::new(4032, 3024);

    fn out(target: Target) -> Dimensions {
        plan(PHOTO, &target, &Options::default()).unwrap().output
    }

    #[test]
    fn vision_examples() {
        assert_eq!(out(Target::Percent(50.0)), Dimensions::new(2016, 1512));
        assert_eq!(out(Target::LongEdge(1920)), Dimensions::new(1920, 1440));
        assert_eq!(out(Target::Megapixels(2.0)), Dimensions::new(1633, 1225));
        let passport = Target::Print {
            width: "35 mm".parse().unwrap(),
            height: "45 mm".parse().unwrap(),
            dpi: 300.0,
            fit: Fit::FillCrop,
        };
        assert_eq!(out(passport), Dimensions::new(413, 531));
        let a4: Length = "210mm".parse().unwrap();
        assert_eq!(a4.to_pixels(300.0), 2480);
    }

    #[test]
    fn fit_modes() {
        let square = Dimensions::new(1080, 1080);
        let p = plan(
            PHOTO,
            &Target::Box {
                size: square,
                fit: Fit::Inside,
            },
            &Options::default(),
        )
        .unwrap();
        assert_eq!(p.output, Dimensions::new(1080, 810));

        let p = plan(
            PHOTO,
            &Target::Box {
                size: square,
                fit: Fit::FillCrop,
            },
            &Options::default(),
        )
        .unwrap();
        assert_eq!(p.scaled, Dimensions::new(1440, 1080));
        assert_eq!(p.output, square);
        assert_eq!(p.offset((0.5, 0.5)), (180, 0));
        assert_eq!(p.offset((0.0, 0.5)), (0, 0));
        assert_eq!(p.offset((1.0, 0.5)), (360, 0));

        let p = plan(
            PHOTO,
            &Target::Box {
                size: square,
                fit: Fit::Pad,
            },
            &Options::default(),
        )
        .unwrap();
        assert_eq!(p.scaled, Dimensions::new(1080, 810));
        assert_eq!(p.output, square);
        assert_eq!(p.offset((0.5, 0.5)), (0, 135));

        let p = plan(
            PHOTO,
            &Target::Box {
                size: square,
                fit: Fit::Stretch,
            },
            &Options::default(),
        )
        .unwrap();
        assert_eq!(p.output, square);
        assert_eq!(p.warnings, vec![Warning::Distorted { percent: 25 }]);
    }

    #[test]
    fn stretch_within_two_percent_is_quiet() {
        let p = plan(
            PHOTO,
            &Target::Box {
                size: Dimensions::new(1000, 740),
                fit: Fit::Stretch,
            },
            &Options::default(),
        )
        .unwrap();
        assert!(p.warnings.is_empty(), "{:?}", p.warnings);
    }

    #[test]
    fn never_enlarge() {
        let small = Dimensions::new(800, 600);
        let p = plan(small, &Target::LongEdge(1920), &Options::default()).unwrap();
        assert_eq!(p.output, small);
        assert_eq!(p.warnings, vec![Warning::KeptOriginalSize]);

        let p = plan(
            small,
            &Target::Percent(300.0),
            &Options {
                never_enlarge: false,
                round_to: 1,
            },
        )
        .unwrap();
        assert_eq!(p.output, Dimensions::new(2400, 1800));
        assert_eq!(p.warnings, vec![Warning::Enlarging { percent: 300 }]);

        let p = plan(
            small,
            &Target::Box {
                size: Dimensions::new(1080, 1080),
                fit: Fit::FillCrop,
            },
            &Options::default(),
        )
        .unwrap();
        assert_eq!(p.output, Dimensions::new(800, 600));
        let p = plan(
            small,
            &Target::Box {
                size: Dimensions::new(1080, 1080),
                fit: Fit::Pad,
            },
            &Options::default(),
        )
        .unwrap();
        assert_eq!(p.output, Dimensions::new(1080, 1080));
    }

    #[test]
    fn rounding_for_video() {
        let p = plan(
            Dimensions::new(1000, 562),
            &Target::Percent(50.0),
            &Options {
                never_enlarge: true,
                round_to: 2,
            },
        )
        .unwrap();
        assert_eq!(p.output, Dimensions::new(500, 282));
    }

    #[test]
    fn rejects_out_of_range() {
        assert!(plan(PHOTO, &Target::Percent(0.5), &Options::default()).is_err());
        assert!(plan(PHOTO, &Target::Percent(401.0), &Options::default()).is_err());
        assert!(plan(PHOTO, &Target::LongEdge(0), &Options::default()).is_err());
    }

    #[test]
    fn parses() {
        assert_eq!(
            "1920x1080".parse::<Dimensions>().unwrap(),
            Dimensions::new(1920, 1080)
        );
        assert_eq!(
            "1920 × 1080".parse::<Dimensions>().unwrap(),
            Dimensions::new(1920, 1080)
        );
        assert!("1920".parse::<Dimensions>().is_err());
        assert!("0x10".parse::<Dimensions>().is_err());
        assert_eq!("4 in".parse::<Length>().unwrap().millimetres(), 101.6);
    }

    #[test]
    fn auto_resampler_choice() {
        let a = Dimensions::new(100, 100);
        assert_eq!(auto_resampler(PHOTO, a, false), Resampler::Lanczos);
        assert_eq!(auto_resampler(a, PHOTO, false), Resampler::Mitchell);
        assert_eq!(
            auto_resampler(Dimensions::new(32, 16), Dimensions::new(128, 64), true),
            Resampler::Nearest
        );
        assert_eq!(
            auto_resampler(Dimensions::new(32, 16), Dimensions::new(100, 50), true),
            Resampler::Mitchell
        );
    }
}
