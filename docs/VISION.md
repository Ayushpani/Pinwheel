# Pinwheel Vision

Pinwheel turns any file drag into a conversion or an edit, on Windows, macOS and Linux. It is free, open source, fully offline, and contains zero AI.

**The promise:** whatever file you are holding, whatever it needs to become, it is one gesture away, and the result lands where you were going.

> Earlier drafts used "Morph" as a working name. This document uses **Pinwheel** throughout.

## Who it is for

- **Everyday people:** HEIC photos that websites reject, attachments that are too big, photos and signatures for forms with strict size rules.
- **Students and office workers:** merging, splitting and shrinking PDFs; turning documents into PDFs; scanning pages cleanly.
- **Creators:** screenshots, thumbnails, social sizes, GIFs, audio clips.
- **Developers and designers:** icon sets, SVG exports, WebP/AVIF, CSV/JSON, fonts.

Why it becomes the go-to place: it is the only one that works on all three systems, is open source, covers images, documents, PDFs, audio, video and archives, and can be extended by its users.

| | Pinwheel | Tangerine | DragShift | File Converter |
|---|---|---|---|---|
| Platforms | Windows, macOS, Linux | macOS 13+, Apple silicon only | Windows | Windows |
| Price and licence | Free, GPL-3 | Paid one-time, closed source | Free, no licence listed | Free, GPL-3 |
| How you start it | Drag + key, hotkey, clipboard, right-click | Drag + Shift | Drag + Shift, right-click | Right-click menu |
| Editing tools | Yes, per file type | Yes | Images and PDF | No |
| Audio and video | Yes | Audio | No | Yes |
| Recipes and custom actions | Yes | Not advertised | No | Custom presets |

Sources: Tangerine review (MacGeneration), Tangerine listing, DragShift, File Converter.

## UX principles

Every feature must pass these ten rules before it ships. If a feature breaks one, it gets redesigned or cut.

1. **One gesture, no window.** At least 8 in 10 actions finish on drop with no dialog. A panel opens only when the action truly needs a choice, like a crop area.
2. **Instant.** The wheel appears within one frame of the trigger key. Waiting is shown as progress around the cursor, never a frozen screen.
3. **Never destroy.** Originals are never changed unless the user asks. Every output can be undone.
4. **Show before you drop.** Hovering a slice shows the expected size and dimensions of the result.
5. **Remember.** The last settings used for each action and file type are kept. Favourite slices can be pinned.
6. **Big targets, close by.** At most 8 slices per ring, each large and near the cursor. Everything else sits one step away in "More".
7. **Keyboard is a first-class citizen.** Every action works without a mouse, and every slice has a screen-reader name.
8. **Feels native on each system.** It follows light/dark mode, accent colour, reduced motion and reduced transparency.
9. **Plain errors with a fix.** "This PDF is locked. Enter its password" beats "Error 0x80004005".
10. **No nagging.** No accounts, no pop-ups, no tips mid-task, no update alerts during work.

## The journey, start to end

A new user should finish their first real conversion within 60 seconds of installing, without reading anything.

1. **Install.** One offline installer per system, everything included. Also on winget, Homebrew and Flathub. No account, no sign-up.
2. **First launch:** a one-window welcome with three steps.
   1. *Try it here.* A practice photo sits inside the window. The user drags it, presses Shift, and drops it on JPG. The result appears beside it with a before/after.
   2. *Allow it to see drags.* macOS needs Accessibility and Input Monitoring permission. One sentence explains why, and a button opens the right settings page.
   3. *Make it yours.* Pick the trigger key (Shift by default) and where results go (next to the original by default). Done.
3. **Invisible by default.** Pinwheel lives in the tray or menu bar. Nothing shows until you drag a file and press the key.
4. **Daily loop.** Drag → key → wheel → hover to preview → drop → result chip at the cursor. The chip can be dragged straight into a browser, email or chat, or copied with one click.
5. **Learning more, gently.** One tip at a time appears inside the result toast, never more than one a day, and they can be switched off. Example: "Tap Shift again for Tools."
6. **Power use.** Recipes, custom actions, folder batches, watch folders and the command-line tool are there when wanted and invisible when not.
7. **Updates.** The app never checks the internet. Package managers update it, or the user downloads a new version. Release notes appear once, after the update.

## The wheel

The wheel is a ring of up to 8 slices around a centre hub, about 280 px across, drawn at the cursor the moment the trigger key is pressed during a drag. Everything a user needs is in the ring and the hub; the flyout and the other two rings appear only when asked for.

**The hub** (centre) shows a thumbnail of the file, its name, size and dimensions or page count. When a slice is hovered, the hub switches to a preview of the result, for example "JPG · about 1.4 MB · 4032 × 3024". Dropping on the hub opens the full panel with every option.

**Three rings, one key.** Tapping the trigger key again cycles the rings in place. Three dots under the hub show which ring is active.

| Ring | What it holds | Example for a photo |
|---|---|---|
| 1. Convert | The 7 most-used target formats for this file type, plus More | JPG, PNG, WebP, AVIF, PDF, TIFF, JPEG XL, More |
| 2. Tools | The 7 most-used tools for this file type, plus More | Resize, Crop, Compress, Adjust, Annotate, Metadata, Add background, More |
| 3. Mine | Fit For targets, recipes and custom actions the user pinned | Exam photo 20–50 KB, Blog image, Signature, Instagram 4:5 |

**Flyouts.** Resting on a slice for 0.4 s opens a small outer arc of variants. For JPG these are Best, Balanced and Smallest. For Resize they are 50%, 1080 px, 1920 px and Custom. Dropping on a variant applies it directly, still with no dialog.

**Keys.** Number keys 1–8 or arrow keys pick a slice, Enter applies, Tab switches ring, Esc closes. Moving the cursor out of the wheel also closes it, and the drag continues as normal.

**Several files at once.** The hub shows a stack with a count, such as "12 files · 3 types". Only actions that work for all of them appear, plus extras: Combine into one PDF, Make a ZIP, Make a collage.

**After the drop.** The wheel shrinks into a progress ring around the cursor. When done, a small result chip stays near the cursor for 6 seconds and a toast appears in the corner.

- The chip can be dragged into any app, such as a browser upload box, which solves "convert, then go find the file".
- The toast shows the change, like "3.1 MB → 412 KB · 87% smaller", with Open, Show in folder, Copy and Undo.

**Look and motion.** Frosted glass using the system material, solid when reduced transparency is on. It pops in from the cursor in 120 ms with a 10 ms stagger per slice, or simply fades when reduced motion is on. Near a screen edge the wheel slides inward so no slice is cut off.

## Every conversion

Pinwheel converts across 12 families of files. All twelve are in version 1.0, built up phase by phase as the roadmap shows.

| Family | From | To | Engine |
|---|---|---|---|
| Photos and images | JPG, PNG, WebP, AVIF, HEIC/HEIF, JPEG XL, TIFF, BMP, GIF, ICO, ICNS, TGA, PSD (flattened), camera RAW (CR2, CR3, NEF, ARW, DNG, RAF, ORF, RW2) | JPG, PNG, WebP, AVIF, JPEG XL, TIFF, BMP, GIF, ICO, ICNS, PDF | libvips, libheif, libraw |
| Vector | SVG | PNG, JPG, WebP, PDF, ICO (any size) | resvg |
| Animated | GIF, animated WebP, APNG, short MP4/WebM | GIF, animated WebP, APNG, MP4, WebM | ffmpeg, libvips |
| PDF | PDF | JPG/PNG/WebP per page, TXT (text layer), DOCX (simple layout), PDF/A; many images → one PDF | PDFium, qpdf |
| Documents | DOCX, ODT, RTF, TXT, Markdown, HTML, EPUB | PDF, DOCX, ODT, Markdown, HTML, TXT, EPUB | pandoc + typst; Word or LibreOffice when installed |
| Spreadsheets and data | XLSX, XLS, ODS, CSV, TSV, JSON, YAML, XML, TOML | CSV, TSV, XLSX, JSON, YAML, Markdown table, HTML table | calamine, rust_xlsxwriter, serde |
| Presentations | PPTX, ODP | PDF, one image per slide | LibreOffice when installed |
| Audio | MP3, M4A/AAC, WAV, FLAC, OGG, Opus, AIFF, ALAC, WMA, AMR | MP3, M4A, WAV, FLAC, OGG, Opus, AIFF, ALAC, M4R ringtone | ffmpeg |
| Video | MP4, MOV, MKV, WebM, AVI, WMV, FLV, MTS/M2TS, 3GP | MP4 (H.264/H.265), WebM (VP9/AV1), MOV, MKV, GIF, audio only, frames as images | ffmpeg |
| Subtitles | SRT, VTT, ASS/SSA | SRT, VTT, ASS, plain text | built in |
| Archives | ZIP, 7Z, TAR, TAR.GZ, TAR.XZ, RAR (extract only), ISO (extract only) | ZIP, ZIP with AES-256 password, 7Z, TAR.GZ, split parts | libarchive, 7-Zip |
| Fonts and small formats | TTF, OTF, WOFF, WOFF2, VCF contacts, ICS calendars | TTF, OTF, WOFF, WOFF2; VCF → CSV; ICS → CSV | woff2, built in |

Two honest limits are shown in the app, not hidden:

- **PDF → Word** keeps text, images and simple layout. Complex layouts will shift, and the toast says so.
- **Office files to PDF** look best when Word or LibreOffice is installed. Without them, DOCX, Markdown and text still convert with the built-in renderer; PPTX and XLSX need one of them.

## Tools ring

The Tools ring changes with the file type. Each tool below is one slice or flyout; the 7 used most appear first, the rest sit in More.

| File type | Tools |
|---|---|
| Images | Resize · Crop and rotate · Compress (by quality or to a size) · Adjust (light, colour, white balance, sharpen, black and white, auto levels) · Annotate (arrow, box, text, numbered steps, highlight, blur, pixelate) · Redact (solid box, burned in) · Add background (padding, rounded corners, shadow, gradient, plain frame) · Watermark (text or logo, tiled or placed) · Remove plain background (colour key, for signatures and logos) · Scan cleanup (4-corner perspective, deskew, whiten paper, black and white) · Metadata (view, remove location, remove all, edit date) · Stitch and collage · Split into tiles · Colour palette |
| PDF | Merge · Split (by range, every N pages, by size) · Page grid (reorder, delete, rotate, duplicate) · Compress (screen, email, print) · Protect and unlock · Sign (saved signature + date) · Fill form · Page numbers · Watermark and stamp · Redact (removes content, not just covers it) · Extract images · Extract pages · Crop margins · Resize pages (A4, Letter) · Greyscale · Flatten · Booklet and 2-up · Metadata |
| Video | Trim · Cut out a section · Compress to a size · Resize or reframe (16:9, 9:16, 1:1; crop or blurred bars) · Rotate and flip · Speed · Mute · Extract audio · Replace audio · Make GIF · Grab frame or thumbnail · Join clips · Burn in subtitles · Fix sideways video · Reverse · Loop |
| Audio | Trim · Fade in and out · Normalise loudness · Speed and pitch · Join · Split on silence · Mono or stereo · Ringtone (30 s with fade) · Tags and cover art · Remove silence at ends |
| Documents and text | Word and character count · Fix encoding to UTF-8 · Line endings (Windows/Unix) · Find and replace across files · Join text files · Markdown to PDF with a theme |
| Spreadsheets and data | Split CSV · Join CSVs · Remove duplicates · Sort · Pick columns · Each sheet to its own CSV · JSON format, minify and validate |
| Archives | Extract here · Extract to folder · Zip with password · Split into parts (for example 25 MB for email) · Test integrity · List contents |
| Any file | Rename (patterns, photo date, numbering) · Fix wrong extension (reads the real type from the file's contents) · Checksum (MD5, SHA-1, SHA-256; compare two) · Copy as Base64 or data URI · Copy path · File info |
| Folders | Convert everything inside, keeping the structure · Find duplicates · Rename all · Zip · Size report |

A tool opens a panel only when it needs input that cannot be guessed: crop area, annotation, signature placement, trim points. Everything else runs on drop with remembered settings.

## Resize and pixel setup

Resize has a one-drop path for common sizes and one panel for everything else. The panel answers three questions on one screen: how many pixels, what print size, what quality.

**Quick path, no panel.** Rest on the Resize slice and its flyout offers 50%, long edge 1080 px, 1920 px, 3840 px and Custom. The first four apply on drop. Custom opens the panel.

### Six ways to set the size

| Mode | You enter | Example |
|---|---|---|
| Percentage | 1–400% | 50% of 4032 × 3024 → 2016 × 1512 |
| Long or short edge | One number in px | Long edge 1920 → 1920 × 1440 |
| Width × height | Both numbers + a fit mode | 1080 × 1080, fill and crop |
| Print size | cm, mm or inches + DPI | 35 × 45 mm at 300 DPI → 413 × 531 px |
| Megapixels | One number | 2 MP at 4:3 → 1633 × 1225 |
| File size | KB or MB, or a range | See [Exact-size targets](#exact-size-targets-and-fit-for) |

Print size uses one formula, shown under the fields so users learn it:

$$\text{pixels} = \frac{\text{millimetres}}{25.4} \times \text{DPI}$$

### Fit modes for width × height

- **Fit inside:** the whole image fits within the box; one side may come out smaller.
- **Fill and crop:** the box is filled and the overflow is cut. The user drags a focal point to choose what stays; the default is the centre.
- **Pad:** the whole image fits, and empty space is filled with a colour, transparency or a blurred copy of the image.
- **Stretch:** exact size, shape distorted. A warning shows if the ratio changes by more than 2%.

Aspect ratio: a lock toggle, plus presets 1:1, 4:5, 3:4, 2:3, 16:9, 9:16, 21:9, A-paper (1:1.414) and custom.

### Pixel setup (collapsed drawer)

| Setting | Options | Default |
|---|---|---|
| Resolution (DPI) | "Change pixels" or "Keep pixels, change print size only" | Change pixels |
| Resampling | Auto, Lanczos, Bicubic, Mitchell, Bilinear, Nearest neighbour | Auto: Lanczos to shrink, Mitchell to enlarge, Nearest for pixel art at whole-number scales |
| Sharpen after shrinking | Off, Light, Medium | Light |
| Never enlarge | On, Off | On |
| Round dimensions to | 1, 2, 4, 8, 16 px | 1 px (2 px for video) |
| Colour profile | Keep, sRGB, Display P3, Greyscale, CMYK for print | Keep; sRGB in web presets |
| Bit depth | 8-bit, 16-bit | Same as the source |
| Transparency | Keep, or flatten onto a colour | Keep where the format allows |
| Orientation | Apply the camera's rotation flag | On, so sideways photos come out upright |
| JPG | Quality 1–100, chroma 4:4:4 or 4:2:0, progressive | 85, 4:2:0, progressive |
| PNG | Lossless, or palette of 2–256 colours with or without dithering | Lossless |
| WebP, AVIF, JPEG XL | Lossless, or quality + encoder speed | Quality 80, balanced speed |
| Canvas | Padding in px or %, extend to a ratio, background colour | None |

### Live preview

- A before/after split slider and a 100% loupe that follows the cursor.
- One line under the preview: output pixels, print size at the chosen DPI, estimated file size.
- Plain warnings, such as "Enlarging past 200% adds blur, not detail" or "Your crop is 4:3 but the target is 1:1".

### Batches and presets

- **Rules for many files:** only shrink files larger than a size, treat portrait and landscape by their long edge, skip files already small enough.
- **Presets:** any panel setup can be saved with a name, such as "Blog 1600w WebP", and pinned as a slice in the Mine ring.

## Exact-size targets and Fit For

Pinwheel hits any file size the user names, either "under X" or "between X and Y", and can check a file against a full upload spec before it is submitted. This is the feature most converters ignore and most people need.

### How size targeting works

1. **Images:** search for the highest quality that fits. If the lowest allowed quality is still too big, shrink the dimensions 5% at a time, never below the spec's minimum. If it still cannot fit, say so plainly.
2. **Too small:** many forms also reject files under a minimum. Pinwheel raises quality first, then offers to enlarge the dimensions, and never pads files with junk data.
3. **PDFs:** re-compress the images inside at stepped resolutions (300, 200, 150, 100 DPI) until the target is met.
4. **Video:** work out the bitrate from the target, then encode in two passes so the result lands just under it.
5. **Audio:** bitrate from size and duration, rounded down to the nearest standard bitrate.

The video formula, with about 3% kept back for the container:

$$\text{video kbps} = 0.97 \times \frac{\text{target size in kilobits}}{\text{duration in seconds}} - \text{audio kbps}$$

KB means different things to different websites. Some count 1,000 bytes, some 1,024. For "under X KB", Pinwheel aims under the stricter 1,000-byte reading, so the file passes either way.

### Fit For presets

A spec card bundles everything one destination asks for: format, pixel size or print size + DPI, file-size range, colour or greyscale, plain background colour, and file name.

| Category | Example cards |
|---|---|
| Forms and IDs | Exam or job application photo, signature, thumb impression, document scan, visa photos by country |
| Messaging and email | Email attachment, WhatsApp, Discord, Telegram |
| Social | Instagram portrait (1080 × 1350), story (1080 × 1920), YouTube thumbnail (1280 × 720) |
| Work and developers | Slack emoji, favicon set, app icon sets for Windows, macOS, Android and iOS |
| Print | A4 at 300 DPI (2480 × 3508 px), US Letter, 4 × 6 in photo prints |

- **Community-maintained:** cards are small text files in a public repository, each with a link to its official source and a "last verified" date. Every release ships the latest verified set; the app itself never downloads anything.
- **Your own cards:** any panel setup can be saved as a card and pinned to the Mine ring.

### Check against a spec

Dropping a file on Check shows every rule as pass or fail, for example "61 KB, too big (max 50 KB) · 200 × 230 px, OK · JPG, OK". One Fix all button applies what is missing.

### Signature helper

A built-in card for the most painful upload: crop, turn the white paper transparent or pure white, darken the ink, then hit the exact pixel size and KB range.

## Recipes, custom actions and history

Recipes chain several steps into one slice, custom actions let users plug in any command-line tool, and history makes every past action repeatable.

### Recipes

- **Builder:** a list of steps the user drags to reorder. Each step is any tool with its settings.
- **Conditions:** a step can run only "if wider than 2000 px", "if larger than 1 MB" or "if it is a PDF".
- **One slice:** a saved recipe appears in the Mine ring like any other action.
- **Shareable:** recipes are plain text files that can be emailed, posted or added to the community repository.

```toml
name = "Blog image"
accepts = ["image"]

[[step]]
tool = "resize"
long_edge = 1600
never_enlarge = true

[[step]]
tool = "metadata"
remove = "location"

[[step]]
tool = "convert"
to = "webp"
max_size = "300 KB"

[output]
name = "{name}-blog"
```

### Custom actions

- A short text file names the action, its icon, the file types it accepts, and the command to run with `{input}` and `{output}` placeholders.
- Before a custom action runs for the first time, Pinwheel shows the exact command and asks for approval.
- Shared recipes that include a custom action are labelled "Runs a program", so nobody runs a command by accident.

### History

- A timeline of every action with thumbnail, before/after size, time and settings, stored locally for 30 days.
- Undo moves the output to the Trash or Recycle Bin.
- Dropping new files onto any history item repeats that exact action on them.

### Command line and watch folders

- The same engine ships as a command-line tool, for example `pinwheel convert photo.heic --to jpg --max-size 200KB` or `pinwheel run "Blog image" ./photos`.
- A watch folder applies a recipe to anything that lands in it, for example every new screenshot becomes a WebP with a dated name.

## Output, naming and safety

Results are saved next to the original under a clear name, nothing is ever overwritten silently, and a crash never leaves a half-written file.

| Rule | Behaviour |
|---|---|
| Where results go | Next to the original (default), a "Pinwheel" subfolder, one fixed folder, ask each time, or replace the original with the old one sent to the Trash (opt-in, per recipe) |
| File names | Template with tokens: `{name}` `{action}` `{date}` `{photo-date}` `{width}` `{height}` `{n}` `{preset}`. Default: `{name}.{ext}`, or `{name}-{action}` when the format stays the same |
| Name clashes | A number is added, as in `photo (2).jpg`; never overwritten |
| Dates | Created and modified dates can be kept from the original |
| Folders | Batch output keeps the folder structure |
| Location data | Kept by default. Sharing presets (messaging, social, Fit For) remove it, and the toast says "Location removed". A setting removes it always |
| Safe writing | Output is written to a temporary file and moved into place only when complete; leftovers are cleaned on next start |
| Long jobs | Video and big batches run in a queue with progress in the tray; any job can be cancelled |
| Result toast | Before → after (size, pixels, pages, duration) with Open, Show in folder, Copy, Undo and the draggable result chip |

## Settings and personalisation

Settings fit on one scrollable page in five groups, and every default works without visiting it.

| Group | What can be changed |
|---|---|
| Trigger | Key: Shift, Ctrl, Alt/Option, Cmd, or mouse button 4. Tap or hold. A delay of 0–300 ms to avoid accidental opens. A list of apps where the wheel never appears, such as games or Photoshop |
| Rings | Which slices appear and in what order, pinned slices, and whether order follows usage (simple counts, not prediction) |
| Defaults | Per format: JPG quality, WebP lossless or not, PDF-to-image DPI, video codec, audio bitrate |
| Look and feel | Wheel size S/M/L, glass or solid, accent colour, labels on or off, sounds, trackpad haptics on Mac |
| Behaviour | Output location, naming template, location-data policy, toast style, history length |

- **Languages:** translatable from day one, including right-to-left languages.
- **Move to a new computer:** all settings, recipes and spec cards export and import as one file.

## Speed budgets and accessibility

Speed is a feature with hard numbers: a change that breaks a budget fails the automated tests and cannot be merged.

| Moment | Budget (target) | How |
|---|---|---|
| Trigger key → wheel visible | 1 frame (16 ms at 60 Hz) | Wheel window created at start-up and kept hidden, not built on demand |
| Hover → size estimate shown | 200 ms | Encode a small sample and scale the result; refine in the background |
| 12 MP photo, HEIC → JPG | 500 ms on a 2020-era laptop | Long-running engine process, libvips streaming |
| Memory when idle | 50 MB | Codecs loaded only when first used |
| CPU when idle | 0% | Event-driven, no polling loops |
| App start | 300 ms | Small native shell; engine starts in parallel |

### Accessibility

- **No-drag path:** a hotkey opens the wheel on the selected files or the clipboard, then arrows and Enter finish the job.
- **Screen readers:** every slice has a spoken name with its result, for example "Convert to JPG, about 340 kilobytes".
- **Vision:** follows high contrast, reduced transparency and system text size; colour never carries meaning alone.
- **Motion:** reduced motion swaps every animation for a simple fade.
- **Targets:** every slice and button is at least 44 × 44 points.
- **One hand:** mouse button 4 can be the trigger, so no keyboard is needed.

## Zero-AI and privacy promise

Pinwheel contains no machine-learning models and makes no network connections, and both promises are checked automatically on every build. The full policy is in [NO-AI.md](../NO-AI.md).

| Common "smart" feature | Why it is out | Pinwheel's classic alternative |
|---|---|---|
| AI background removal | Needs a neural network | Colour-key removal for plain backgrounds (signatures, logos, product shots on white) |
| AI upscaling | Needs a neural network | Honest resampling with a warning past 200% |
| OCR (text from images) | Modern Tesseract uses an LSTM network | Text is extracted only from PDFs that already contain it |
| Smart or face-aware crop | Needs detection models | User drags the focal point |
| AI auto-enhance | Needs a model | Auto levels from the histogram, fully predictable |

## Architecture

Only the part that must feel instant and native is written three times; everything else is written once and shared.

```
 ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
 │ Windows shell│  │ macOS shell  │  │ Linux shell  │   native: spot drags,
 │  (native)    │  │  (native)    │  │  (native)    │   draw the wheel, show results
 └──────┬───────┘  └──────┬───────┘  └──────┬───────┘
        └─────────────────┼─────────────────┘
                          ▼
                ┌───────────────────┐      ┌──────────────┐
                │  Pinwheel engine  │◄─────┤ pinwheel CLI │
                │      (Rust)       │      └──────────────┘
                └───────────────────┘
                 libvips · ffmpeg · PDFium · qpdf · pandoc · …
```

- **Why native shells:** spotting a drag in another app and drawing a wheel within one frame needs each system's own tools. A web view cannot do either.
- **Why a Rust engine:** fast, memory-safe, and one codebase that builds for all three systems and the command line.
- **Licence:** ffmpeg with H.264/H.265 encoders and libimagequant are GPL, so Pinwheel is GPL-3. MuPDF and Ghostscript (AGPL) are avoided.

## Roadmap

Pinwheel is built in six phases, and each one starts only when the gate before it is passed, so quality never trails features.

The first usable release is the end of phase 2: one system, the wheel, and conversions. Durations depend on team size, so they are left open until that is known.

The six phases, with the scope and gate for each, are in [ROADMAP.md](ROADMAP.md).

## How we'll know it's the best

With no telemetry, success is measured by automated tests, small user studies and public numbers, never by watching users.

| Measure | How it is measured | Target for 1.0 |
|---|---|---|
| First conversion, unaided | 5 new users per release, observed in person or on a call | All 5 within 60 s of install |
| Actions finished with no dialog | Count across the standard test scenarios | 8 in 10 or more |
| Accidental wheel opens | One hour of normal use per tester | Zero |
| Speed budgets | Benchmark suite in CI on every change | All budgets met |
| Correctness | Test library of real files for every format and tool | Every output checked for format, size, pages and metadata |
| Size targets | Test library run against every Fit For card | Lands in range every time, or a clear reason why not |
| Reach | GitHub stars, Homebrew and Flathub install counts, winget downloads | Tracked per release |
| Community | Shared recipes, spec cards and translations | Tracked per release |
