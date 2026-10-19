//! File sizes as people write them: "300 KB", "1.5MB", "25 MiB", "20–50 KB".
//!
//! Websites disagree on what a kilobyte is: some count 1,000 bytes, some
//! 1,024. Pinwheel always takes the stricter reading, so a file passes either
//! way: an upper limit uses the smaller reading and a lower limit the larger.

use std::fmt;
use std::str::FromStr;

use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    B,
    KB,
    MB,
    GB,
    KiB,
    MiB,
    GiB,
}

impl Unit {
    /// The smallest and largest number of bytes this unit can mean.
    fn readings(self) -> (u64, u64) {
        match self {
            Unit::B => (1, 1),
            Unit::KB => (1_000, 1 << 10),
            Unit::MB => (1_000_000, 1 << 20),
            Unit::GB => (1_000_000_000, 1 << 30),
            Unit::KiB => (1 << 10, 1 << 10),
            Unit::MiB => (1 << 20, 1 << 20),
            Unit::GiB => (1 << 30, 1 << 30),
        }
    }

    fn parse(s: &str) -> Option<Unit> {
        Some(match s.to_ascii_lowercase().as_str() {
            "" | "b" | "byte" | "bytes" => Unit::B,
            "k" | "kb" => Unit::KB,
            "m" | "mb" => Unit::MB,
            "g" | "gb" => Unit::GB,
            "kib" => Unit::KiB,
            "mib" => Unit::MiB,
            "gib" => Unit::GiB,
            _ => return None,
        })
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Unit::B => "B",
            Unit::KB => "KB",
            Unit::MB => "MB",
            Unit::GB => "GB",
            Unit::KiB => "KiB",
            Unit::MiB => "MiB",
            Unit::GiB => "GiB",
        })
    }
}

/// A size as the user wrote it, before choosing which reading of the unit applies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SizeSpec {
    pub value: f64,
    pub unit: Unit,
}

impl SizeSpec {
    /// The byte count to stay at or under for "under X": the smaller reading.
    pub fn max_bytes(&self) -> u64 {
        (self.value * self.unit.readings().0 as f64).floor() as u64
    }

    /// The byte count to reach for "at least X": the larger reading.
    pub fn min_bytes(&self) -> u64 {
        (self.value * self.unit.readings().1 as f64).ceil() as u64
    }
}

impl FromStr for SizeSpec {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        let t = s.trim();
        let split = t
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(t.len());
        let (num, unit) = t.split_at(split);
        let value: f64 = num.parse().map_err(|_| Error::InvalidSize(s.into()))?;
        let unit = Unit::parse(unit.trim()).ok_or_else(|| Error::InvalidSize(s.into()))?;
        if !value.is_finite() || value < 0.0 {
            return Err(Error::InvalidSize(s.into()));
        }
        Ok(SizeSpec { value, unit })
    }
}

impl fmt::Display for SizeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.value, self.unit)
    }
}

/// An allowed range of file sizes, in bytes, with the strict readings applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SizeRange {
    pub min: Option<u64>,
    pub max: Option<u64>,
}

impl SizeRange {
    pub fn under(max: SizeSpec) -> Self {
        SizeRange {
            min: None,
            max: Some(max.max_bytes()),
        }
    }

    pub fn contains(&self, bytes: u64) -> bool {
        self.min.is_none_or(|m| bytes >= m) && self.max.is_none_or(|m| bytes <= m)
    }
}

impl FromStr for SizeRange {
    type Err = Error;

    /// Accepts "50 KB", "under 50 KB", "at least 20 KB" and "20-50 KB"
    /// (the lower bound may leave out its unit).
    fn from_str(s: &str) -> Result<Self, Error> {
        let t = s.trim();
        let lower = t.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("under ") {
            return Ok(SizeRange::under(rest.parse()?));
        }
        if let Some(rest) = lower.strip_prefix("at least ") {
            let min: SizeSpec = rest.parse()?;
            return Ok(SizeRange {
                min: Some(min.min_bytes()),
                max: None,
            });
        }
        let Some((a, b)) = t.split_once(['-', '–']) else {
            return Ok(SizeRange::under(t.parse()?));
        };
        let max: SizeSpec = b.parse().map_err(|_| Error::InvalidSize(s.into()))?;
        let mut min: SizeSpec = a.parse().map_err(|_| Error::InvalidSize(s.into()))?;
        if !a.trim().ends_with(|c: char| c.is_ascii_alphabetic()) {
            min.unit = max.unit;
        }
        let range = SizeRange {
            min: Some(min.min_bytes()),
            max: Some(max.max_bytes()),
        };
        if range.min > range.max {
            return Err(Error::InvalidSize(s.into()));
        }
        Ok(range)
    }
}

/// A size the way the result toast shows it: "412 KB", "3.1 MB".
pub fn format_bytes(bytes: u64) -> String {
    let b = bytes as f64;
    if bytes < 1_000 {
        format!("{bytes} B")
    } else if bytes < 999_500 {
        format!("{} KB", (b / 1e3).round())
    } else if bytes < 999_950_000 {
        format!("{:.1} MB", b / 1e6)
    } else {
        format!("{:.1} GB", b / 1e9)
    }
}

/// The toast line for a size change: "3.1 MB → 412 KB · 87% smaller".
pub fn describe_change(before: u64, after: u64) -> String {
    let arrow = format!("{} → {}", format_bytes(before), format_bytes(after));
    if before == 0 || before == after {
        return arrow;
    }
    let ratio = after as f64 / before as f64;
    if after < before {
        format!("{arrow} · {}% smaller", ((1.0 - ratio) * 100.0).round())
    } else {
        format!("{arrow} · {}% larger", ((ratio - 1.0) * 100.0).round())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_spellings() {
        for (s, value, unit) in [
            ("300 KB", 300.0, Unit::KB),
            ("1.5MB", 1.5, Unit::MB),
            ("200kb", 200.0, Unit::KB),
            ("25 MiB", 25.0, Unit::MiB),
            ("512", 512.0, Unit::B),
        ] {
            assert_eq!(
                s.parse::<SizeSpec>().unwrap(),
                SizeSpec { value, unit },
                "{s}"
            );
        }
        assert!("fifty KB".parse::<SizeSpec>().is_err());
        assert!("50 KX".parse::<SizeSpec>().is_err());
    }

    #[test]
    fn upper_limits_use_the_stricter_reading() {
        let kb50: SizeSpec = "50 KB".parse().unwrap();
        assert_eq!(kb50.max_bytes(), 50_000);
        assert_eq!(kb50.min_bytes(), 51_200);
    }

    #[test]
    fn parses_ranges() {
        let r: SizeRange = "20–50 KB".parse().unwrap();
        assert_eq!(r.min, Some(20_480));
        assert_eq!(r.max, Some(50_000));
        assert!(r.contains(30_000));
        assert!(
            !r.contains(20_000),
            "20,000 bytes fails a site counting 1,024"
        );
        assert!(!r.contains(50_001));

        let r: SizeRange = "500 KB-2 MB".parse().unwrap();
        assert_eq!(r.min, Some(512_000));
        assert_eq!(r.max, Some(2_000_000));

        assert_eq!(
            "under 200KB".parse::<SizeRange>().unwrap().max,
            Some(200_000)
        );
        assert_eq!(
            "at least 1 MB".parse::<SizeRange>().unwrap().min,
            Some(1_048_576)
        );
        assert!("50-20 KB".parse::<SizeRange>().is_err());
    }

    #[test]
    fn formats_like_the_toast() {
        assert_eq!(format_bytes(999), "999 B");
        assert_eq!(format_bytes(412_000), "412 KB");
        assert_eq!(format_bytes(999_600), "1.0 MB");
        assert_eq!(format_bytes(3_100_000), "3.1 MB");
        assert_eq!(
            describe_change(3_100_000, 412_000),
            "3.1 MB → 412 KB · 87% smaller"
        );
        assert_eq!(
            describe_change(100_000, 150_000),
            "100 KB → 150 KB · 50% larger"
        );
    }
}
