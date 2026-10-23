//! Draws the wheel into a premultiplied RGBA bitmap. Kept free of Windows
//! calls so it can be tested and previewed anywhere.

use ab_glyph::{point, Font, FontVec, PxScale, ScaleFont};
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};

use crate::geometry::{slice_angle, Hit, HUB, OUTER, SIZE, SLICES};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub dark: bool,
    /// The system accent colour, used for the hovered slice.
    pub accent: [u8; 3],
    /// Reduced transparency is on: draw the wheel solid.
    pub solid: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            dark: false,
            accent: [0, 103, 192],
            solid: false,
        }
    }
}

pub struct Fonts {
    pub regular: FontVec,
    pub bold: FontVec,
}

impl Fonts {
    pub fn from_bytes(regular: Vec<u8>, bold: Vec<u8>) -> Option<Fonts> {
        Some(Fonts {
            regular: FontVec::try_from_vec(regular).ok()?,
            bold: FontVec::try_from_vec(bold).ok()?,
        })
    }
}

/// Everything the wheel shows at one moment.
pub struct View<'a> {
    pub labels: &'a [&'a str],
    pub hovered: Hit,
    pub hub_title: &'a str,
    pub hub_detail: &'a str,
    /// Grey out the slices: the dragged files have nothing the ring can do.
    pub disabled: bool,
}

struct Palette {
    surface: [u8; 4],
    hub: [u8; 4],
    line: [u8; 4],
    ink: [u8; 4],
    muted: [u8; 4],
    shadow: [u8; 3],
}

fn palette(theme: &Theme) -> Palette {
    let glass = if theme.solid { 255 } else { 236 };
    if theme.dark {
        Palette {
            surface: [36, 38, 48, glass],
            hub: [26, 28, 36, 255],
            line: [255, 255, 255, 26],
            ink: [242, 243, 247, 255],
            muted: [166, 170, 186, 255],
            shadow: [0, 0, 0],
        }
    } else {
        Palette {
            surface: [250, 250, 252, glass],
            hub: [238, 240, 245, 255],
            line: [20, 24, 40, 24],
            ink: [24, 26, 38, 255],
            muted: [92, 97, 116, 255],
            shadow: [16, 20, 40],
        }
    }
}

fn paint(rgba: [u8; 4]) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(Color::from_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]));
    p.anti_alias = true;
    p
}

/// Black or white, whichever reads better on `bg`.
fn on(bg: [u8; 3]) -> [u8; 4] {
    let l = 0.2126 * bg[0] as f32 + 0.7152 * bg[1] as f32 + 0.0722 * bg[2] as f32;
    if l > 150.0 {
        [0, 0, 0, 255]
    } else {
        [255, 255, 255, 255]
    }
}

fn sector(c: f32, r_in: f32, r_out: f32, a0: f32, a1: f32) -> Option<tiny_skia::Path> {
    const STEPS: usize = 24;
    let mut pb = PathBuilder::new();
    for i in 0..=STEPS {
        let a = a0 + (a1 - a0) * i as f32 / STEPS as f32;
        let (x, y) = (c + r_out * a.cos(), c + r_out * a.sin());
        if i == 0 {
            pb.move_to(x, y)
        } else {
            pb.line_to(x, y)
        }
    }
    for i in (0..=STEPS).rev() {
        let a = a0 + (a1 - a0) * i as f32 / STEPS as f32;
        pb.line_to(c + r_in * a.cos(), c + r_in * a.sin());
    }
    pb.close();
    pb.finish()
}

fn circle(c: f32, r: f32) -> Option<tiny_skia::Path> {
    PathBuilder::from_circle(c, c, r)
}

/// Width of `text` at `size` pixels.
fn measure(font: &FontVec, size: f32, text: &str) -> f32 {
    let f = font.as_scaled(PxScale::from(size));
    let mut w = 0.0;
    let mut prev = None;
    for ch in text.chars() {
        let id = f.glyph_id(ch);
        if let Some(p) = prev {
            w += f.kern(p, id);
        }
        w += f.h_advance(id);
        prev = Some(id);
    }
    w
}

/// Shortens `text` with an ellipsis in the middle until it fits, keeping the
/// extension visible: "a-very-long-na…me.jpg".
fn fit(font: &FontVec, size: f32, text: &str, max: f32) -> String {
    if measure(font, size, text) <= max {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let keep_end = text
        .rfind('.')
        .map_or(4, |i| text[i..].chars().count() + 2)
        .min(chars.len());
    for start in (1..chars.len().saturating_sub(keep_end)).rev() {
        let candidate: String = chars[..start]
            .iter()
            .chain(['…'].iter())
            .chain(chars[chars.len() - keep_end..].iter())
            .collect();
        if measure(font, size, &candidate) <= max {
            return candidate;
        }
    }
    "…".to_string()
}

/// Draws `text` centred on `cx`, with its baseline at `baseline`.
fn text(
    pixmap: &mut Pixmap,
    font: &FontVec,
    size: f32,
    s: &str,
    cx: f32,
    baseline: f32,
    rgba: [u8; 4],
) {
    let scaled = font.as_scaled(PxScale::from(size));
    let mut x = (cx - measure(font, size, s) / 2.0).round();
    let mut prev = None;
    let (w, h) = (pixmap.width() as i32, pixmap.height() as i32);
    let data = pixmap.data_mut();
    for ch in s.chars() {
        let id = scaled.glyph_id(ch);
        if let Some(p) = prev {
            x += scaled.kern(p, id);
        }
        let glyph = id.with_scale_and_position(PxScale::from(size), point(x, baseline));
        x += scaled.h_advance(id);
        prev = Some(id);
        let Some(outline) = font.outline_glyph(glyph) else {
            continue;
        };
        let b = outline.px_bounds();
        outline.draw(|gx, gy, coverage| {
            let px = b.min.x as i32 + gx as i32;
            let py = b.min.y as i32 + gy as i32;
            if px < 0 || py < 0 || px >= w || py >= h {
                return;
            }
            let a = coverage.clamp(0.0, 1.0) * rgba[3] as f32 / 255.0;
            let i = ((py * w + px) * 4) as usize;
            for k in 0..3 {
                let src = rgba[k] as f32 * a;
                data[i + k] = (src + data[i + k] as f32 * (1.0 - a)).round() as u8;
            }
            data[i + 3] = (a * 255.0 + data[i + 3] as f32 * (1.0 - a)).round() as u8;
        });
    }
}

/// Renders the wheel at `scale` (1.0 at 96 DPI). The bitmap is square,
/// `SIZE × scale` pixels, premultiplied RGBA.
pub fn render(view: &View, theme: &Theme, fonts: &Fonts, scale: f32) -> Pixmap {
    let px = (SIZE * scale).round() as u32;
    let mut pixmap = Pixmap::new(px, px).expect("non-zero size");
    let c = px as f32 / 2.0;
    let (outer, hub) = (OUTER * scale, HUB * scale);
    let pal = palette(theme);
    let id = Transform::identity();

    // A soft shadow: rings fading outward, a little below the wheel.
    for i in (1..=12).rev() {
        let alpha = (if theme.dark { 7 } else { 4 }) as f32 * (13 - i) as f32 / 12.0;
        if let Some(p) = PathBuilder::from_circle(c, c + 3.0 * scale, outer + i as f32 * scale) {
            let s = pal.shadow;
            pixmap.fill_path(
                &p,
                &paint([s[0], s[1], s[2], alpha as u8]),
                FillRule::Winding,
                id,
                None,
            );
        }
    }

    if let Some(p) = circle(c, outer) {
        pixmap.fill_path(&p, &paint(pal.surface), FillRule::Winding, id, None);
        let stroke = Stroke {
            width: scale,
            ..Stroke::default()
        };
        pixmap.stroke_path(&p, &paint(pal.line), &stroke, id, None);
    }

    let half = std::f32::consts::PI / SLICES as f32;
    if let (Hit::Slice(i), false) = (view.hovered, view.disabled) {
        let a = slice_angle(i);
        let gap = 0.012;
        if let Some(p) = sector(
            c,
            hub + 4.0 * scale,
            outer - 4.0 * scale,
            a - half + gap,
            a + half - gap,
        ) {
            let [r, g, b] = theme.accent;
            pixmap.fill_path(&p, &paint([r, g, b, 255]), FillRule::Winding, id, None);
        }
    }

    let stroke = Stroke {
        width: scale,
        ..Stroke::default()
    };
    for i in 0..SLICES {
        let a = slice_angle(i) - half;
        let mut pb = PathBuilder::new();
        pb.move_to(
            c + (hub + 10.0 * scale) * a.cos(),
            c + (hub + 10.0 * scale) * a.sin(),
        );
        pb.line_to(
            c + (outer - 10.0 * scale) * a.cos(),
            c + (outer - 10.0 * scale) * a.sin(),
        );
        if let Some(p) = pb.finish() {
            pixmap.stroke_path(&p, &paint(pal.line), &stroke, id, None);
        }
    }

    let label_size = 14.0 * scale;
    for (i, label) in view.labels.iter().enumerate().take(SLICES) {
        let a = slice_angle(i);
        let r = (hub + outer) / 2.0 + 2.0 * scale;
        let (x, y) = (c + r * a.cos(), c + r * a.sin());
        let hot = view.hovered == Hit::Slice(i) && !view.disabled;
        let color = if hot {
            on(theme.accent)
        } else if view.disabled {
            pal.muted
        } else {
            pal.ink
        };
        text(
            &mut pixmap,
            &fonts.bold,
            label_size,
            label,
            x,
            y + label_size * 0.36,
            color,
        );
    }

    if let Some(p) = circle(c, hub) {
        pixmap.fill_path(&p, &paint(pal.hub), FillRule::Winding, id, None);
        pixmap.stroke_path(&p, &paint(pal.line), &stroke, id, None);
    }
    let max = (hub * 2.0) - 16.0 * scale;
    let title = fit(&fonts.bold, 12.5 * scale, view.hub_title, max);
    text(
        &mut pixmap,
        &fonts.bold,
        12.5 * scale,
        &title,
        c,
        c - 3.0 * scale,
        pal.ink,
    );
    let detail = fit(&fonts.regular, 11.0 * scale, view.hub_detail, max);
    text(
        &mut pixmap,
        &fonts.regular,
        11.0 * scale,
        &detail,
        c,
        c + 13.0 * scale,
        pal.muted,
    );

    pixmap
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn test_fonts() -> Option<Fonts> {
        let read = |paths: &[&str]| paths.iter().find_map(|p| std::fs::read(p).ok());
        Fonts::from_bytes(
            read(&[
                "C:/Windows/Fonts/segoeui.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            ])?,
            read(&[
                "C:/Windows/Fonts/seguisb.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
            ])?,
        )
    }

    fn pixel(p: &Pixmap, x: u32, y: u32) -> [u8; 4] {
        let c = p.pixel(x, y).unwrap();
        [c.red(), c.green(), c.blue(), c.alpha()]
    }

    #[test]
    fn draws_hover_in_the_accent_colour() {
        let Some(fonts) = test_fonts() else { return };
        let labels = [
            "JPG", "PNG", "WebP", "ICO", "500 KB", "50%", "1920 px", "TIFF",
        ];
        let theme = Theme {
            accent: [200, 30, 30],
            ..Theme::default()
        };
        let view = View {
            labels: &labels,
            hovered: Hit::Slice(2),
            hub_title: "beach.png",
            hub_detail: "5.6 MB",
            disabled: false,
        };
        let p = render(&view, &theme, &fonts, 1.0);
        assert_eq!((p.width(), p.height()), (312, 312));
        let c = 156;
        // Inside slice 2 (right), away from its label.
        assert_eq!(pixel(&p, c + 120, c - 12), [200, 30, 30, 255]);
        // Slice 6 (left) is plain glass.
        let left = pixel(&p, c - 120, c - 12);
        assert!(
            (236..255).contains(&left[3]),
            "glass should be see-through: {left:?}"
        );
        // The corners are see-through.
        assert_eq!(pixel(&p, 0, 0)[3], 0);
    }

    #[test]
    fn scales_with_dpi() {
        let Some(fonts) = test_fonts() else { return };
        let view = View {
            labels: &[],
            hovered: Hit::Outside,
            hub_title: "",
            hub_detail: "",
            disabled: false,
        };
        assert_eq!(render(&view, &Theme::default(), &fonts, 1.5).width(), 468);
    }

    #[test]
    fn long_names_keep_their_extension() {
        let Some(fonts) = test_fonts() else { return };
        let s = fit(
            &fonts.bold,
            12.5,
            "IMG_20260412_183355_holiday_beach.jpg",
            90.0,
        );
        assert!(s.contains('…') && s.ends_with(".jpg"), "{s}");
        assert!(measure(&fonts.bold, 12.5, &s) <= 90.0);
        assert_eq!(fit(&fonts.bold, 12.5, "a.jpg", 90.0), "a.jpg");
    }
}
