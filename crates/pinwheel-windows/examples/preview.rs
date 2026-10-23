//! Renders the wheel to PNG files, to look at it without Windows:
//! `cargo run -p pinwheel-windows --example preview -- <out-dir>`

use pinwheel_windows::geometry::Hit;
use pinwheel_windows::render::{render, Fonts, Theme, View};
use pinwheel_windows::slices::image_ring;

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let read = |p: &str| std::fs::read(p).expect("font");
    let fonts = Fonts::from_bytes(
        read("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
        read("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"),
    )
    .expect("fonts");
    let ring = image_ring();
    let labels: Vec<&str> = ring.iter().map(|s| s.label).collect();
    for (name, dark, hovered, title, detail) in [
        (
            "light",
            false,
            Hit::Slice(2),
            "IMG_20260412_183355_holiday.png",
            "Convert to WebP",
        ),
        ("dark", true, Hit::Slice(4), "Holiday", "JPG under 500 KB"),
        ("idle", false, Hit::Outside, "12 files", "Drop on a slice"),
    ] {
        let theme = Theme {
            dark,
            ..Theme::default()
        };
        let view = View {
            labels: &labels,
            hovered,
            hub_title: title,
            hub_detail: detail,
            disabled: false,
        };
        render(&view, &theme, &fonts, 2.0)
            .save_png(format!("{out}/wheel-{name}.png"))
            .expect("write png");
    }
}
