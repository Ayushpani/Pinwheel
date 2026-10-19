# Pinwheel Roadmap

How Pinwheel goes from today's engine core to a 1.0 that ships on Windows, macOS and Linux and lives up to [the vision](VISION.md).

Each of the six phases ends with a **gate**: a list of things that must be true before the next phase starts. No phase has a date; dates come once the team size is known.

## Where we are

| Done | Not started |
|---|---|
| Engine logic: resize planning, size targeting, naming, recipes, spec cards, file-type detection | Codec-backed conversions (libvips, ffmpeg, PDFium, …) |
| CLI: `info`, `checksum`, `fix-ext`, `check`, `recipe`, `plan` | The wheel and the native shells |
| CI on three systems, zero-AI check, GPL-3 | Installers, signing, package managers, website |

## Decisions to make first

These change the plan, so settle them before Phase 1 ends.

| Decision | Recommendation | Why |
|---|---|---|
| First system to ship | **Windows** | Biggest audience for the core users (form photos, HEIC from iPhones, oversized attachments). It needs no permission prompts to watch drags, so first-launch is simplest. macOS follows in Phase 4. |
| How shells talk to the engine | **Separate engine process, JSON messages over stdin/stdout** | A crash in a codec can't take down the wheel, it adds no sockets (keeps the no-network promise easy to prove), and the CLI uses the same engine. |
| How codecs are shipped | **Link libvips, libheif, libraw, qpdf and PDFium as libraries; run ffmpeg as a bundled program** | Libraries give the 500 ms HEIC → JPG budget. Running ffmpeg as its own program keeps video jobs cancellable and isolated. |
| HEIC and H.265 (patents) | **Use the system's own codecs where they exist** (Windows HEVC extension, macOS ImageIO and AVFoundation) and the bundled libheif/x265 only where the law allows | HEVC is covered by patents in several countries. Get advice before bundling an encoder in the Windows and macOS builds. |
| Documents to PDF | **typst (Rust) built in. Use pandoc only if it is installed, or ship it as an optional add-on** | pandoc adds about 150 MB to the installer. |
| RAR archives | **Extract with libarchive only** | The official unRAR licence is not GPL-compatible. |
| Name | **Check the trademark and domain for "Pinwheel" now** | Renaming after launch costs the package names, the links and the stars. |
| Costs | Apple Developer ($99/year), Windows code signing (Azure Trusted Signing, about $10/month), a domain | Unsigned apps get blocked by Gatekeeper and SmartScreen, which breaks "first conversion in 60 seconds". |

## Phase 1: Foundations and spikes

Prove the three hardest things before building on them.

**Engine**
- Add a `pinwheel-engine` crate: a long-running process with a job queue, progress, cancellation, and safe writing (write to a temporary file, then move it into place).
- Define the shell ↔ engine protocol (`convert`, `estimate`, `probe`, `cancel`, `undo`) in one shared schema file, versioned from day one.
- First real conversions behind `pinwheel convert`: JPG, PNG, WebP, AVIF, HEIC → JPG/PNG/WebP/AVIF/PDF through libvips.
- Run the image search from `target::fit_image` with real encoders, and `pinwheel convert --max-size` on top of it.

**Spikes (throwaway code, one per system)**
- Windows: a low-level mouse and keyboard hook spots "button held + Shift pressed". A topmost window that is already created and hidden appears at the cursor and accepts the drop on each slice (`IDropTarget`). Measure key → wheel time.
- macOS: a listen-only `CGEventTap` with the drag pasteboard, an `NSPanel` already created and hidden, and an `NSVisualEffectView` for the glass. Confirm that this works inside the App Sandbox with Input Monitoring.
- Linux X11: XDND drop target plus XInput2 key watching.
- **Linux Wayland: the biggest risk.** Wayland does not let apps watch global keys during another app's drag. Try the GlobalShortcuts portal and a layer-shell surface. Prepare a fallback: a drop zone at the screen edge that turns into the wheel.

**Quality base**
- Test library of real files for every format: freely licensed or generated, stored with Git LFS, with expected format, size, dimensions and metadata for each output.
- Benchmark suite (criterion) for the speed budgets. Run it on fixed hardware so the numbers are stable.
- Fuzz every parser that reads untrusted text: recipes, spec cards and file-type detection.

**Gate**
- On every system, including Wayland or its fallback: the wheel shows within 16 ms of the key during a drag from the file manager and from a browser, and a drop delivers the file paths.
- HEIC → JPG of a 12 MP photo in under 500 ms on a 2020-era laptop.
- The same input and settings give byte-identical output on every run.
- Every decision in the table above is made and written down.

## Phase 2: First usable release (Windows)

The vision's first milestone: one system, the wheel, and conversions.

**Shell (Windows, Rust + windows-rs, DirectComposition for the wheel)**
- Wheel: 8 slices, hub with thumbnail and file facts, the Convert ring, flyouts after 0.4 s, keyboard control (1–8, arrows, Enter, Esc), edge avoidance, Mica/acrylic glass that turns solid when transparency is reduced, and simple fades when motion is reduced.
- Hover preview: an estimated size and dimensions within 200 ms (encode a small sample and scale the result).
- After the drop: a progress ring, a result chip that can be dragged into any app for 6 seconds, and a toast with before → after plus Open, Show in folder, Copy and Undo.
- Several files at once: a stack in the hub, only the actions that fit every file, and Combine into one PDF and Make a ZIP.
- Tray icon, a settings page (the five groups), and a list of apps where the wheel never appears.
- First launch: the three-step welcome with a practice photo.
- Screen-reader names for every slice (UI Automation), plus high contrast.

**Engine**
- Conversions: all image formats including RAW and SVG, PDF ↔ images, audio and video basics through ffmpeg.
- Undo sends outputs to the Recycle Bin. History is kept locally in SQLite for 30 days.
- Output naming, clash numbering, location-data policy and keeping file dates, all from `pinwheel-core`.

**Shipping**
- Signed MSI or MSIX, a winget manifest, and a portable ZIP.
- Website: one page with a demo GIF, downloads, the zero-AI promise, and a link to the source.
- End-to-end tests run with networking blocked by the firewall.

**Gate**
- 5 new users, watched in person or on a call, each finish their first conversion within 60 s of install.
- 8 in 10 or more of the standard scenarios finish with no dialog.
- Zero accidental wheel opens in an hour of normal use per tester.
- Every speed budget passes in CI.

## Phase 3: Tools ring and exact sizes

The part most converters skip and most people need.

- Tools ring per file type, with the top 7 by use and the rest in More. Images and PDF first: Resize, Crop and rotate, Compress, Adjust, Metadata, Merge, Split, Page grid, Protect, Sign, Redact (redaction removes the content, not just covers it).
- The Resize panel: six modes, four fit modes, a focal point, the pixel-setup drawer, a before/after slider, a loupe, and plain warnings.
- Exact-size targets for images, PDFs (the 300/200/150/100 DPI steps), video (two passes) and audio.
- Fit For: a spec-card library in a separate public repository (`pinwheel-cards`), with Check, Fix all, and the Signature helper. Every card has a source link and a "last verified" date, and CI checks each card's format.
- The Mine ring, pinned slices, and remembering the last settings per action and file type.

**Gate:** every card in the library lands in range on the test library, or gives a clear reason why not. The vision's UX principles are re-checked against every new panel.

## Phase 4: macOS and Linux

The engine is shared, so this phase is only shells, packaging and parity.

- macOS shell in Swift and AppKit: menu-bar app, permission onboarding for Accessibility and Input Monitoring with a button to the right settings page, trackpad haptics, and VoiceOver. Signed with Developer ID, notarized, sandboxed with no network entitlement, and on a Homebrew cask.
- Linux shell in GTK4 (gtk-rs): tray via StatusNotifierItem, Orca names, the Wayland approach chosen in Phase 1. Flatpak on Flathub with no network permission, plus AppImage, .deb and .rpm.
- One end-to-end scenario list, run on all three systems.

**Gate:** the Phase 2 gate passes again on macOS and Linux, and the three systems produce identical engine output for the same inputs.

## Phase 5: Every family and every power tool

- The remaining families: documents (typst, plus Word or LibreOffice when installed), spreadsheets and data, presentations, subtitles, archives (with AES-256 ZIP and split parts), fonts, VCF and ICS.
- Video and audio tools from the vision's table, and the Any file and Folders tools.
- Recipes: the builder with drag-to-reorder and conditions, share as a file, import with a preview.
- Custom actions: show the exact command and ask for approval before the first run. Shared recipes that run a program carry a "Runs a program" label.
- History you can drop new files on to repeat an action, watch folders, and the full CLI (`pinwheel convert`, `pinwheel run "Blog image" ./photos`).
- Export and import of all settings, recipes and cards as one file.

**Gate:** the test library covers every format and tool in the vision, with outputs checked for format, size, pages and metadata.

## Phase 6: 1.0 polish and launch

- **Security:** Pinwheel opens untrusted files with large C libraries, so decode in a sandboxed worker (AppContainer on Windows, App Sandbox on macOS, seccomp and Landlock on Linux). Pin and track codec versions for security fixes, publish a SECURITY.md, and get an outside review of the custom-action approval flow.
- **Accessibility audit** on each system: keyboard-only, screen reader, high contrast, large text, reduced motion, one hand on mouse button 4.
- **Languages:** strings in Fluent files shared by every shell, community translation through Weblate, and right-to-left layout checked.
- **Reproducible builds:** publish checksums, and let anyone rebuild and compare.
- **Docs:** user guide, CLI reference, recipe and card format guides, CONTRIBUTING, a code of conduct, and issue templates.
- **Launch:**
  - Release on winget, Homebrew, Flathub and the website the same day.
  - A 30-second demo video.
  - Show HN, Product Hunt, and the subreddits for each audience.
  - Reach out to the reviewers who covered the competitors.

**Gate (1.0):** every target in the vision's "How we'll know it's the best" table is met.

## After 1.0

Track reach and community per release (stars, install counts, shared recipes, cards and translations). Keep the gates: a feature that breaks a speed budget or a UX principle does not merge.

## Next five concrete steps

1. Make the decisions in the table above, starting with the first system and the name check.
2. Build the `pinwheel-engine` crate and the protocol, with libvips image conversions behind `pinwheel convert`.
3. Run the drag-detection spikes, Wayland first, because it is the most likely to change the design.
4. Start the test library and the benchmark suite.
5. Open the Apple Developer and Windows signing accounts, which can take weeks to approve.
