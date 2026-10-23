//! What each slice of the wheel does.

use pinwheel_core::resize::{Options, Target};
use pinwheel_core::units::SizeRange;
use pinwheel_engine::batch::Action;
use pinwheel_engine::image_ops::Format;

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Convert(Format),
    Resize(Target),
    /// JPG under this size.
    Under(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Slice {
    /// Shown on the slice.
    pub label: &'static str,
    /// Shown in the hub while the slice is hovered, and read by screen readers.
    pub hint: &'static str,
    /// Title of the result notification.
    pub done: &'static str,
    pub kind: Kind,
}

/// The ring for images and folders of images, clockwise from the top.
pub fn image_ring() -> Vec<Slice> {
    let convert = |label, hint, done, f| Slice {
        label,
        hint,
        done,
        kind: Kind::Convert(f),
    };
    vec![
        convert("JPG", "Convert to JPG", "Converted to JPG", Format::Jpg),
        convert("PNG", "Convert to PNG", "Converted to PNG", Format::Png),
        convert("WebP", "Convert to WebP", "Converted to WebP", Format::Webp),
        convert(
            "ICO",
            "Make an icon, up to 256 px",
            "Made an icon",
            Format::Ico,
        ),
        Slice {
            label: "500 KB",
            hint: "JPG under 500 KB",
            done: "Made smaller",
            kind: Kind::Under("500 KB"),
        },
        Slice {
            label: "50%",
            hint: "Resize to half",
            done: "Resized to half",
            kind: Kind::Resize(Target::Percent(50.0)),
        },
        Slice {
            label: "1920 px",
            hint: "Resize, long edge 1920 px",
            done: "Resized to 1920 px",
            kind: Kind::Resize(Target::LongEdge(1920)),
        },
        convert("TIFF", "Convert to TIFF", "Converted to TIFF", Format::Tiff),
    ]
}

impl Slice {
    /// The engine action, and the size to aim for if any.
    pub fn job(&self) -> (Action, Option<SizeRange>) {
        match &self.kind {
            Kind::Convert(to) => (Action::Convert { to: *to }, None),
            Kind::Resize(target) => (
                Action::Resize {
                    target: target.clone(),
                    options: Options::default(),
                    to: None,
                },
                None,
            ),
            Kind::Under(size) => (
                Action::Convert { to: Format::Jpg },
                Some(
                    format!("under {size}")
                        .parse()
                        .expect("valid built-in size"),
                ),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::SLICES;

    #[test]
    fn ring_fills_the_wheel() {
        let ring = image_ring();
        assert_eq!(ring.len(), SLICES);
        for s in &ring {
            assert!(
                s.label.chars().count() <= 7,
                "{} is too long for a slice",
                s.label
            );
            s.job();
        }
        assert_eq!(ring[4].job().1.unwrap().max, Some(500_000));
    }
}
