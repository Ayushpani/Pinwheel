# Pinwheel for Windows: Features

Everything Windows users get, and when. Windows ships first and is tested end to end; Linux follows with the same engine, then macOS.

| Label | Meaning |
|---|---|
| **Works today** | Built and tested in the engine. Usable now from the `pinwheel` command line. |
| **Release 1** | The first public Windows release. |
| **Release 2** | The next Windows update, before work moves to Linux. |

## Bulk work

| Feature | When |
|---|---|
| Bulk conversion: many files or whole folders in one go | Works today |
| Bulk resize: one size for everything, by %, long or short edge, width × height, print size or megapixels | Works today |
| Subfolders included, folder structure kept | Works today |
| Non-image files in a folder left alone and counted ("Skipped 1 file that is not an image") | Works today |
| One bad file never stops the batch; each failure has a plain reason | Works today |
| **ZIP after bulk conversion or resize**: "Also make a ZIP", or "ZIP only" with no loose files. Named after the folder (`Holiday-webp.zip`) and saved beside it | Works today |
| Totals at the end: "3 files · 14.7 MB → 1.1 MB · 92% smaller" | Works today |
| Never enlarge (on by default) | Works today |
| Uses every processor core | Works today |
| Mixed file types: the wheel shows only the actions that work for all of them, plus Combine into one PDF and Make a ZIP | Release 1 |
| Batch rules: only shrink files bigger than a size, skip files already small enough | Release 1 |
| Progress in the tray; cancel any batch | Release 1 |

## Where results go

| Feature | When |
|---|---|
| **Beside the original** (the default) | Works today |
| In a subfolder named "Pinwheel" (or a name you choose); never re-processed on the next run | Works today |
| **A folder you choose**, keeping the dropped folder's structure | Works today |
| Never overwrites: `photo (2).jpg` | Works today |
| Safe writing: a crash never leaves a half-written file | Works today |
| Name templates such as `{name}-small` or `{name}_{width}x{height}` | Works today |
| Ask each time, with the Windows folder picker | Release 1 |
| Change the location for one batch from the wheel's hub, or set the default in Settings | Release 1 |
| **Drag the result chip** into any folder, browser upload box, email or chat | Release 1 |
| Undo: sends the result to the Recycle Bin | Release 1 |
| Replace the original (opt-in), with the original sent to the Recycle Bin | Release 2 |

## Ways to start

| Feature | When |
|---|---|
| Command line: `pinwheel convert`, `pinwheel resize` and more | Works today |
| Drag a file, press Shift: the wheel appears at the cursor | Release 1 |
| Right-click in File Explorer: Pinwheel menu | Release 1 |
| Select files in Explorer and press a hotkey: the wheel without dragging | Release 1 |
| Drop files on the tray icon | Release 1 |
| Copy a screenshot and press the hotkey: convert the clipboard | Release 2 |

## The wheel

| Feature | When |
|---|---|
| Up to 8 big slices around a hub with the file's thumbnail, name, size and dimensions | Release 1 |
| Three rings, one key: Convert, Tools, Mine | Release 1 |
| Hover a slice to see the result before dropping: "JPG · about 1.4 MB · 4032 × 3024" | Release 1 |
| Flyouts after 0.4 s: Best, Balanced, Smallest; 50%, 1080 px, 1920 px, Custom | Release 1 |
| Keyboard: 1–8 or arrows, Enter, Tab, Esc | Release 1 |
| Result chip for 6 seconds and a toast with Open, Show in folder, Copy and Undo | Release 1 |
| Follows light/dark mode, accent colour, reduced motion and reduced transparency | Release 1 |

## Conversions

| Family | Formats | When |
|---|---|---|
| Images | JPG, PNG, WebP, GIF, BMP, TIFF, ICO | Works today |
| Images | HEIC/HEIF (Windows' own codec), AVIF, JPEG XL, camera RAW, PSD, TGA, ICNS | Release 1 |
| Vector | SVG to PNG, JPG, WebP, PDF, ICO | Release 1 |
| PDF | PDF to images per page, images to one PDF, text layer to TXT | Release 1 |
| Audio | MP3, M4A, WAV, FLAC, OGG, Opus, AIFF and more; M4R ringtones | Release 1 |
| Video | MP4, MOV, MKV, WebM, AVI and more; to GIF, audio only, frames | Release 1 |
| Animated | GIF, animated WebP, APNG, short MP4/WebM | Release 2 |
| Documents | DOCX, ODT, RTF, TXT, Markdown, HTML, EPUB to PDF and back | Release 2 |
| Spreadsheets and data | XLSX, XLS, ODS, CSV, TSV, JSON, YAML, XML, TOML | Release 2 |
| Presentations | PPTX, ODP to PDF or images (needs Word or LibreOffice) | Release 2 |
| Subtitles | SRT, VTT, ASS | Release 2 |
| Archives | ZIP, 7Z, TAR, TAR.GZ; extract RAR and ISO; password ZIP; split parts | Release 2 |
| Fonts and contacts | TTF, OTF, WOFF, WOFF2; VCF and ICS to CSV | Release 2 |

## Tools

| File type | Release 1 | Release 2 |
|---|---|---|
| Images | Resize, Crop and rotate, Compress, Metadata (remove location) | Adjust, Annotate, Redact, Add background, Watermark, Remove plain background, Scan cleanup, Collage, Tiles, Palette |
| PDF | Merge, Split, Compress, Page grid | Protect and unlock, Sign, Fill form, Page numbers, Watermark, Redact, Extract, Crop margins, Resize pages, Greyscale, Flatten, Booklet |
| Video | Trim, Compress to a size, Extract audio, Make GIF | Cut out, Reframe, Rotate, Speed, Mute, Replace audio, Frames, Join, Subtitles, Reverse, Loop |
| Audio | Trim, Normalise loudness, Ringtone | Fade, Speed and pitch, Join, Split on silence, Mono/stereo, Tags and cover art |
| Any file | Info, Checksum, Fix wrong extension (all work today on the command line) | Rename patterns, Copy as Base64, Copy path |
| Folders | Convert everything inside (works today) | Find duplicates, Rename all, Size report |

## Exact sizes and Fit For

| Feature | When |
|---|---|
| Compress to a size: "under 50 KB" or "20–50 KB" (JPG, WebP); quality first, then dimensions 5% at a time | Works today |
| Strict KB: aims under 1,000-byte KB so files pass sites that count either way | Works today |
| Check a file against a spec card: "61 KB, too big (max 50 KB) · 200 × 230 px, OK · JPG, OK" | Works today |
| Fit For library: exam and job photos, signatures, visa photos, WhatsApp, Instagram, YouTube, print sizes | Release 1 |
| Fix all: one click applies what a spec card is missing | Release 1 |
| Signature helper: crop, white or transparent paper, darker ink, exact size | Release 2 |
| Video and audio to a size | Release 2 |

## Recipes, history and automation

| Feature | When |
|---|---|
| Recipe file format, with validation | Works today |
| History of the last 30 days, with before/after sizes | Release 1 |
| Run recipes from the Mine ring; build them by dragging steps | Release 2 |
| Repeat any past action by dropping files on it | Release 2 |
| Watch folders | Release 2 |
| Custom actions (any command-line tool), shown and approved before the first run | Release 2 |

## Privacy, safety and access

| Feature | When |
|---|---|
| No internet connections, no AI, no accounts, no telemetry, checked on every build | Works today |
| Originals are never changed | Works today |
| Screen-reader names for every slice, full keyboard use, high contrast, 44 × 44 pt targets | Release 1 |
| Mouse button 4 as the trigger, for one-handed use | Release 1 |
| Each file opened in a sandboxed worker | Release 2 |

## Getting it, free

| Feature | When |
|---|---|
| Free signed installer (SignPath Foundation), so SmartScreen does not block it | Release 1 |
| winget, Microsoft Store and a portable ZIP | Release 1 |
| Settings, recipes and cards export and import as one file | Release 2 |

## How it is tested on Windows

- **Every change:** the full test suite runs on a real Windows machine in GitHub Actions. That includes end-to-end batch tests that convert, resize and ZIP real image files in real folders.
- **Before each release:** a hands-on pass on a Windows PC, covering Explorer, browsers, Outlook, WhatsApp Desktop, OneDrive folders, long paths, non-English file names, high contrast and a screen reader.
- **Release gate:** 5 new users each finish a first conversion within 60 seconds of installing.

## Known gaps, and when they close

| Gap today | Fix | When |
|---|---|---|
| Photo metadata (date taken, camera, location) is not carried over yet | Keep metadata by default; remove location only in sharing presets | Release 1 |
| Running a second batch on the same folder also picks up the first batch's results | History remembers what Pinwheel made and skips it | Release 1 |
| HEIC (iPhone photos) cannot be opened yet | Use Windows' own HEIF codec | Release 1 |
| A 12 MP photo takes longer than the 500 ms budget | Move image work to libvips | Release 1 |
| No wheel yet: the command line only | Build the Windows shell | Release 1 |
