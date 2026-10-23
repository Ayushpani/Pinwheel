//! Fit For spec cards: everything one upload destination asks for, and a
//! check that shows each rule as pass or fail.
//!
//! ```toml
//! name = "Exam photo"
//! source = "https://example.org/official-photo-rules"
//! verified = "2026-09-01"
//! format = ["jpg"]
//! width = 200
//! height = 230
//! size = "20-50 KB"
//! ```

use serde::Deserialize;

use crate::filetype::canonical_ext;
use crate::recipe::FileFacts;
use crate::units::{format_bytes, SizeRange};
use crate::Error;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecCard {
    pub name: String,
    /// Link to the destination's official rules.
    pub source: Option<String>,
    /// When someone last checked the card against its source.
    pub verified: Option<String>,
    #[serde(default)]
    pub format: Vec<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub min_width: Option<u32>,
    pub max_width: Option<u32>,
    pub min_height: Option<u32>,
    pub max_height: Option<u32>,
    /// "50 KB", "under 2 MB" or "20-50 KB".
    pub size: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub passed: bool,
    pub message: String,
}

impl SpecCard {
    pub fn from_toml(text: &str) -> Result<SpecCard, Error> {
        let card: SpecCard =
            toml::from_str(text).map_err(|e| Error::Spec(e.message().to_string()))?;
        if card.name.trim().is_empty() {
            return Err(Error::Spec("it needs a name.".into()));
        }
        if let Some(size) = &card.size {
            size.parse::<SizeRange>()
                .map_err(|e| Error::Spec(e.to_string()))?;
        }
        Ok(card)
    }

    fn has_pixel_rules(&self) -> bool {
        [
            self.width,
            self.height,
            self.min_width,
            self.max_width,
            self.min_height,
            self.max_height,
        ]
        .iter()
        .any(Option::is_some)
    }

    /// Every rule on the card, checked against `file`, in a fixed order:
    /// file size, pixel size, format.
    pub fn check(&self, file: &FileFacts) -> Vec<Check> {
        let mut checks = Vec::new();

        if let Some(text) = &self.size {
            let range: SizeRange = text.parse().unwrap_or_default();
            let size = format_bytes(file.bytes);
            let bound = if range.min.is_some() {
                format!("allowed {}", text.trim())
            } else {
                format!("max {}", text.trim().trim_start_matches("under ").trim())
            };
            checks.push(match (range.max, range.min) {
                (Some(max), _) if file.bytes > max => fail(format!("{size}, too big ({bound})")),
                (_, Some(min)) if file.bytes < min => fail(format!("{size}, too small ({bound})")),
                _ => pass(format!("{size}, OK")),
            });
        }

        if self.has_pixel_rules() {
            checks.push(match file.dimensions {
                None => fail("Could not read the size in pixels".into()),
                Some(d) => {
                    let mut wrong = Vec::new();
                    let rules = [
                        (self.width, d.width, "wide", 0),
                        (self.height, d.height, "high", 0),
                        (self.min_width, d.width, "wide", 1),
                        (self.min_height, d.height, "high", 1),
                        (self.max_width, d.width, "wide", -1),
                        (self.max_height, d.height, "high", -1),
                    ];
                    for (rule, got, axis, dir) in rules {
                        let Some(want) = rule else { continue };
                        let bad = match dir {
                            0 => got != want,
                            1 => got < want,
                            _ => got > want,
                        };
                        if bad {
                            let how = match dir {
                                0 => "",
                                1 => "at least ",
                                _ => "at most ",
                            };
                            wrong.push(format!("should be {how}{want} px {axis}"));
                        }
                    }
                    if wrong.is_empty() {
                        pass(format!("{d} px, OK"))
                    } else {
                        fail(format!("{d} px, {}", wrong.join(" and ")))
                    }
                }
            });
        }

        if !self.format.is_empty() {
            let ext = canonical_ext(&file.ext);
            let shown = ext.to_ascii_uppercase();
            checks.push(if self.format.iter().any(|f| canonical_ext(f) == ext) {
                pass(format!("{shown}, OK"))
            } else {
                let wanted: Vec<String> = self
                    .format
                    .iter()
                    .map(|f| canonical_ext(f).to_ascii_uppercase())
                    .collect();
                fail(format!("{shown}, should be {}", wanted.join(" or ")))
            });
        }

        checks
    }
}

fn pass(message: String) -> Check {
    Check {
        passed: true,
        message,
    }
}

fn fail(message: String) -> Check {
    Check {
        passed: false,
        message,
    }
}

/// The one-line summary: "61 KB, too big (max 50 KB) · 200 × 230 px, OK · JPG, OK".
pub fn summary(checks: &[Check]) -> String {
    checks
        .iter()
        .map(|c| c.message.as_str())
        .collect::<Vec<_>>()
        .join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resize::Dimensions;

    const EXAM: &str = r#"
name = "Exam photo"
format = ["jpg"]
width = 200
height = 230
size = "50 KB"
"#;

    fn file(bytes: u64, w: u32, h: u32, ext: &str) -> FileFacts {
        FileFacts {
            ext: ext.into(),
            bytes,
            dimensions: Some(Dimensions::new(w, h)),
        }
    }

    #[test]
    fn vision_example() {
        let card = SpecCard::from_toml(EXAM).unwrap();
        let checks = card.check(&file(61_000, 200, 230, "jpeg"));
        assert_eq!(
            summary(&checks),
            "61 KB, too big (max 50 KB) · 200 × 230 px, OK · JPG, OK"
        );
        assert_eq!(checks.iter().filter(|c| !c.passed).count(), 1);
    }

    #[test]
    fn reports_every_failure() {
        let card = SpecCard::from_toml(&EXAM.replace("\"50 KB\"", "\"20-50 KB\"")).unwrap();
        let checks = card.check(&file(12_000, 300, 230, "png"));
        assert_eq!(
            summary(&checks),
            "12 KB, too small (allowed 20-50 KB) · 300 × 230 px, should be 200 px wide · PNG, should be JPG"
        );
        assert!(checks.iter().all(|c| !c.passed));
    }

    #[test]
    fn ranges_of_pixels() {
        let card = SpecCard {
            name: "x".into(),
            min_width: Some(1000),
            max_height: Some(800),
            ..SpecCard::default()
        };
        let checks = card.check(&file(1, 900, 900, "jpg"));
        assert_eq!(
            checks[0].message,
            "900 × 900 px, should be at least 1000 px wide and should be at most 800 px high"
        );
    }

    #[test]
    fn rejects_bad_cards() {
        assert!(SpecCard::from_toml("name = \"x\"\nsize = \"tiny\"").is_err());
        assert!(SpecCard::from_toml("name = \"x\"\ncolour = \"red\"").is_err());
    }
}
