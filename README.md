# Pinwheel

Pinwheel turns any file drag into a conversion or an edit, on Windows, macOS and Linux. It is free, open source, fully offline, and contains zero AI.

**The promise:** whatever file you are holding, whatever it needs to become, it is one gesture away, and the result lands where you were going.

Drag a file, press Shift, and a wheel of choices appears at the cursor: convert to JPG, shrink to under 200 KB, resize for Instagram, merge into one PDF. Drop on a slice and the result appears next to the original, ready to drag into a browser, email or chat.

- **Everything:** images, camera RAW, SVG, PDFs, documents, spreadsheets, audio, video, subtitles, archives and fonts.
- **Exact sizes:** "under 50 KB" or "20–50 KB" for forms, with checks against a destination's full upload spec.
- **Yours:** recipes chain steps into one slice; custom actions plug in any command-line tool.
- **Private:** no accounts, no telemetry, no network connections, no machine-learning models. See [NO-AI.md](NO-AI.md).

Read the full product vision in [docs/VISION.md](docs/VISION.md).

## Status

Early development. This repository currently holds the shared engine core and the command-line tool. The native shells (the wheel itself) and the codec-backed conversions are not built yet.

| Part | State |
|---|---|
| `pinwheel-core`: resize planning, size targeting, output naming, recipes, spec cards, file-type detection | Working, tested |
| `pinwheel` CLI: `info`, `checksum`, `fix-ext`, `check`, `recipe`, `plan` | Working |
| Conversions (libvips, ffmpeg, PDFium, …) | Not started |
| Native shells for Windows, macOS and Linux | Not started |

## Try the command-line tool

```sh
cargo run -p pinwheel-cli -- --help

# What is this file really?
cargo run -p pinwheel-cli -- info photo.jpg

# Will it pass the upload form?
cargo run -p pinwheel-cli -- check photo.jpg --card exam-photo.toml
# 61 KB, too big (max 50 KB) · 200 × 230 px, OK · JPG, OK

# Plan a passport photo: 35 × 45 mm at 300 DPI
cargo run -p pinwheel-cli -- plan resize 4032x3024 --print-width "35 mm" --print-height "45 mm" --dpi 300 --fit fill-crop
# 4032 × 3024 → 413 × 531 px

# Bitrate for a 25 MB, two-minute video
cargo run -p pinwheel-cli -- plan video --size 25MB --duration 120
```

## Layout

```
crates/pinwheel-core   engine logic shared by every shell and the CLI
crates/pinwheel-cli    the `pinwheel` command
docs/VISION.md         what Pinwheel is and the rules every feature must pass
scripts/no-ai-check.sh CI check that keeps ML models and networking out
```

## Contributing

Run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `scripts/no-ai-check.sh` before opening a pull request. Every feature must pass the ten UX principles in the vision, and nothing may add a model or a network connection.

## Licence

GPL-3.0-or-later. See [LICENSE](LICENSE).
