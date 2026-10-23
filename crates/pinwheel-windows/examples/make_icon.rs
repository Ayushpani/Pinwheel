//! Draws the Pinwheel icon at every size Windows asks for and writes
//! `assets/pinwheel.ico`: `cargo run -p pinwheel-windows --example make_icon`

use image::codecs::ico::{IcoEncoder, IcoFrame};
use image::ExtendedColorType;
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Transform};

/// Four blades, clockwise from the top: blue, teal, amber, coral.
const BLADES: [[u8; 3]; 4] = [[43, 87, 221], [18, 150, 140], [240, 170, 40], [232, 86, 70]];

fn draw(px: u32) -> Pixmap {
    let mut pixmap = Pixmap::new(px, px).expect("size");
    let s = px as f32 / 32.0;
    let c = px as f32 / 2.0;
    for (i, rgb) in BLADES.iter().enumerate() {
        // One blade, turned into place: up from the centre, across to the
        // corner of the square the paper was cut from, then a bowed edge
        // back to the centre. Four of them make the swirl.
        let mut pb = PathBuilder::new();
        pb.move_to(0.0, -1.0);
        pb.line_to(0.0, -15.0);
        pb.line_to(13.0, -15.0);
        pb.quad_to(5.0, -9.5, 0.9, -1.4);
        pb.close();
        let path = pb.finish().expect("blade");
        let t = Transform::from_rotate(90.0 * i as f32)
            .post_scale(s, s)
            .post_translate(c, c);
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba8(rgb[0], rgb[1], rgb[2], 255));
        paint.anti_alias = true;
        pixmap.fill_path(&path, &paint, FillRule::Winding, t, None);
    }
    let mut hub = Paint::default();
    hub.set_color(Color::from_rgba8(255, 255, 255, 255));
    hub.anti_alias = true;
    if let Some(p) = PathBuilder::from_circle(c, c, 2.6 * s) {
        pixmap.fill_path(&p, &hub, FillRule::Winding, Transform::identity(), None);
    }
    pixmap
}

fn main() {
    let frames: Vec<IcoFrame> = [16u32, 20, 24, 32, 40, 48, 64, 256]
        .iter()
        .map(|&px| {
            // ICO wants straight (not premultiplied) alpha.
            let rgba: Vec<u8> = draw(px)
                .pixels()
                .iter()
                .flat_map(|p| {
                    let c = p.demultiply();
                    [c.red(), c.green(), c.blue(), c.alpha()]
                })
                .collect();
            IcoFrame::as_png(&rgba, px, px, ExtendedColorType::Rgba8).expect("frame")
        })
        .collect();
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/pinwheel.ico");
    let file = std::fs::File::create(&out).expect("create icon");
    IcoEncoder::new(file)
        .encode_images(&frames)
        .expect("write icon");
    draw(256)
        .save_png(out.with_extension("png"))
        .expect("preview");
    println!("wrote {}", out.display());
}
