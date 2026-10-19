//! `pinwheel`: the Pinwheel engine on the command line.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use md5::Md5;
use sha1::Sha1;
use sha2::{Digest, Sha256};

use pinwheel_core::filetype::{canonical_ext, family_of, sniff, wrong_extension};
use pinwheel_core::naming::{unique_path, validate_template};
use pinwheel_core::recipe::{FileFacts, Recipe};
use pinwheel_core::resize::{self, Dimensions, Fit, Length, Target};
use pinwheel_core::spec::{summary, SpecCard};
use pinwheel_core::target;
use pinwheel_core::units::{describe_change, format_bytes, SizeRange, SizeSpec};
use pinwheel_engine::batch::{self, Action, Report, Settings, ZipMode};
use pinwheel_engine::image_ops::Format;
use pinwheel_engine::output::Location;

/// Bytes read from the start of a file to tell what it is.
const SNIFF_BYTES: u64 = 64 * 1024;

#[derive(Parser)]
#[command(
    name = "pinwheel",
    version,
    about = "Convert and edit any file. Offline, open source, zero AI."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show what a file really is, its size and its pixel size
    Info { files: Vec<PathBuf> },
    /// Print a checksum, or compare two files
    Checksum {
        file: PathBuf,
        #[arg(long, value_enum, default_value_t = Algo::Sha256)]
        algo: Algo,
        /// Another file to compare against
        #[arg(long)]
        compare: Option<PathBuf>,
    },
    /// Find files whose extension does not match their contents
    FixExt {
        files: Vec<PathBuf>,
        /// Rename them (never overwriting anything) instead of only reporting
        #[arg(long)]
        apply: bool,
    },
    /// Check a file against a Fit For spec card
    Check {
        file: PathBuf,
        #[arg(long)]
        card: PathBuf,
    },
    /// Convert images or whole folders to another format
    Convert {
        /// Files and folders
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        /// Target format: jpg, png, webp, gif, bmp, tiff or ico
        #[arg(long)]
        to: FormatArg,
        #[command(flatten)]
        output: OutputArgs,
    },
    /// Resize images or whole folders
    Resize {
        /// Files and folders
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        #[command(flatten)]
        size: SizeArgs,
        /// Also convert to this format (keeps each file's format if left out)
        #[arg(long)]
        to: Option<FormatArg>,
        #[command(flatten)]
        output: OutputArgs,
    },
    /// Validate recipe files
    Recipe { files: Vec<PathBuf> },
    /// Work out a result before making it
    #[command(subcommand)]
    Plan(Plan),
}

#[derive(Subcommand)]
enum Plan {
    /// Output pixels for a resize
    Resize {
        /// Source size, such as 4032x3024
        from: Dimensions,
        #[command(flatten)]
        size: SizeArgs,
    },
    /// Video bitrate that lands just under a file size
    Video {
        #[arg(long)]
        size: SizeSpec,
        /// Duration in seconds
        #[arg(long)]
        duration: f64,
        /// Audio bitrate in kbps
        #[arg(long, default_value_t = 128)]
        audio: u32,
    },
    /// Audio bitrate that lands under a file size
    Audio {
        #[arg(long)]
        size: SizeSpec,
        /// Duration in seconds
        #[arg(long)]
        duration: f64,
    },
}

#[derive(Args)]
#[group(id = "mode", required = true, multiple = false)]
struct Mode {
    /// Percentage, 1–400
    #[arg(long)]
    percent: Option<f64>,
    #[arg(long)]
    long_edge: Option<u32>,
    #[arg(long)]
    short_edge: Option<u32>,
    /// Width × height box, such as 1080x1080
    #[arg(long = "box")]
    box_size: Option<Dimensions>,
    /// Print width, such as "35 mm" (with --print-height and --dpi)
    #[arg(long, requires_all = ["print_height", "dpi"])]
    print_width: Option<Length>,
    #[arg(long)]
    megapixels: Option<f64>,
}

#[derive(Args)]
struct SizeArgs {
    #[command(flatten)]
    mode: Mode,
    #[arg(long)]
    print_height: Option<Length>,
    #[arg(long)]
    dpi: Option<f64>,
    #[arg(long, value_enum, default_value_t = FitArg::Inside)]
    fit: FitArg,
    /// Allow the result to be bigger than the source
    #[arg(long)]
    allow_enlarge: bool,
    /// Round dimensions to a multiple of this
    #[arg(long, default_value_t = 1)]
    round_to: u32,
}

#[derive(Args)]
struct OutputArgs {
    /// Save results in this folder (keeps the structure of dropped folders)
    #[arg(long, value_name = "FOLDER", conflicts_with = "subfolder")]
    into: Option<PathBuf>,
    /// Save results in a subfolder [default name: Pinwheel]
    #[arg(long, value_name = "NAME", num_args = 0..=1, default_missing_value = "Pinwheel")]
    subfolder: Option<String>,
    /// Also put all results in one ZIP
    #[arg(long, conflicts_with = "zip_only")]
    zip: bool,
    /// Only make the ZIP, with no loose files
    #[arg(long)]
    zip_only: bool,
    /// Quality 1–100 for JPG and WebP
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..=100))]
    quality: Option<u8>,
    /// Aim for a file size, such as "200 KB" or "20-50 KB" (JPG and WebP)
    #[arg(long)]
    max_size: Option<SizeRange>,
    /// Name template, such as "{name}-small" (tokens: {name} {action} {width} {height} {ext})
    #[arg(long)]
    name: Option<String>,
}

impl OutputArgs {
    fn settings(self) -> Result<Settings, String> {
        if let Some(t) = &self.name {
            validate_template(t).map_err(|e| e.to_string())?;
        }
        let location = match (self.into, self.subfolder) {
            (Some(dir), _) => Location::Folder(dir),
            (None, Some(name)) => Location::Subfolder(name),
            (None, None) => Location::BesideOriginal,
        };
        let zip = match (self.zip, self.zip_only) {
            (_, true) => ZipMode::Only,
            (true, false) => ZipMode::Also,
            _ => ZipMode::Off,
        };
        Ok(Settings {
            quality: self.quality,
            max_size: self.max_size,
            location,
            template: self.name,
            zip,
        })
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum FormatArg {
    #[value(alias = "jpeg")]
    Jpg,
    Png,
    Webp,
    Gif,
    Bmp,
    #[value(alias = "tif")]
    Tiff,
    Ico,
}

impl From<FormatArg> for Format {
    fn from(f: FormatArg) -> Format {
        match f {
            FormatArg::Jpg => Format::Jpg,
            FormatArg::Png => Format::Png,
            FormatArg::Webp => Format::Webp,
            FormatArg::Gif => Format::Gif,
            FormatArg::Bmp => Format::Bmp,
            FormatArg::Tiff => Format::Tiff,
            FormatArg::Ico => Format::Ico,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum FitArg {
    Inside,
    FillCrop,
    Pad,
    Stretch,
}

impl From<FitArg> for Fit {
    fn from(f: FitArg) -> Fit {
        match f {
            FitArg::Inside => Fit::Inside,
            FitArg::FillCrop => Fit::FillCrop,
            FitArg::Pad => Fit::Pad,
            FitArg::Stretch => Fit::Stretch,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum Algo {
    Md5,
    Sha1,
    Sha256,
}

fn main() -> ExitCode {
    match run(Cli::parse().command) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// Ok(false) means the command worked but found a problem (a failed check).
fn run(command: Command) -> Result<bool, String> {
    match command {
        Command::Info { files } => {
            for path in &files {
                let facts = facts(path)?;
                let real = sniff(&head(path)?);
                let kind = real
                    .map(str::to_string)
                    .unwrap_or_else(|| canonical_ext(&facts.ext));
                let family = family_of(&kind).map_or("unknown".into(), |f| f.to_string());
                let mut line = format!(
                    "{}: {} ({family}) · {}",
                    path.display(),
                    kind.to_ascii_uppercase(),
                    format_bytes(facts.bytes)
                );
                if let Some(d) = facts.dimensions {
                    line.push_str(&format!(" · {d} px"));
                }
                println!("{line}");
            }
            Ok(true)
        }
        Command::Checksum {
            file,
            algo,
            compare,
        } => {
            let a = checksum(&file, algo)?;
            let Some(other) = compare else {
                println!("{a}  {}", file.display());
                return Ok(true);
            };
            let b = checksum(&other, algo)?;
            let same = a == b;
            println!("{}", if same { "Identical" } else { "Different" });
            println!("{a}  {}\n{b}  {}", file.display(), other.display());
            Ok(same)
        }
        Command::FixExt { files, apply } => {
            let mut all_right = true;
            for path in &files {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let Some(real) = wrong_extension(ext, &head(path)?) else {
                    continue;
                };
                all_right = false;
                if !apply {
                    println!(
                        "{}: is really {}",
                        path.display(),
                        real.to_ascii_uppercase()
                    );
                    continue;
                }
                let dir = path.parent().unwrap_or(Path::new(""));
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
                let to = unique_path(dir, &format!("{stem}.{real}"), |p| p.exists());
                fs::rename(path, &to)
                    .map_err(|e| format!("Could not rename {}: {e}", path.display()))?;
                println!("{} → {}", path.display(), to.display());
            }
            Ok(all_right || apply)
        }
        Command::Check { file, card } => {
            let text = read_text(&card)?;
            let card =
                SpecCard::from_toml(&text).map_err(|e| format!("{}: {e}", card.display()))?;
            let checks = card.check(&facts(&file)?);
            println!("{}", summary(&checks));
            Ok(checks.iter().all(|c| c.passed))
        }
        Command::Recipe { files } => {
            let mut ok = true;
            for path in &files {
                match Recipe::from_toml(&read_text(path)?) {
                    Ok(r) => {
                        let label = if r.runs_program() {
                            " · Runs a program"
                        } else {
                            ""
                        };
                        println!(
                            "{}: \"{}\", {} steps{label}",
                            path.display(),
                            r.name,
                            r.steps.len()
                        );
                    }
                    Err(e) => {
                        ok = false;
                        println!("{}: {e}", path.display());
                    }
                }
            }
            Ok(ok)
        }
        Command::Convert { inputs, to, output } => {
            let action = Action::Convert { to: to.into() };
            print_report(&batch::run(&inputs, &action, &output.settings()?)?)
        }
        Command::Resize {
            inputs,
            size,
            to,
            output,
        } => {
            let (target, options) = size.target()?;
            let action = Action::Resize {
                target,
                options,
                to: to.map(Into::into),
            };
            print_report(&batch::run(&inputs, &action, &output.settings()?)?)
        }
        Command::Plan(Plan::Resize { from, size }) => {
            let (target, options) = size.target()?;
            let plan = resize::plan(from, &target, &options).map_err(|e| e.to_string())?;
            println!("{from} → {} px", plan.output);
            for w in &plan.warnings {
                println!("{w}");
            }
            Ok(true)
        }
        Command::Plan(Plan::Video {
            size,
            duration,
            audio,
        }) => {
            let kbps =
                target::video_kbps(size.max_bytes(), duration, audio).map_err(|e| e.to_string())?;
            println!("Video {kbps} kbps + audio {audio} kbps, two passes");
            Ok(true)
        }
        Command::Plan(Plan::Audio { size, duration }) => {
            let kbps = target::audio_kbps(size.max_bytes(), duration).map_err(|e| e.to_string())?;
            println!("Audio {kbps} kbps");
            Ok(true)
        }
    }
}

impl SizeArgs {
    fn target(self) -> Result<(Target, resize::Options), String> {
        let m = self.mode;
        let fit = self.fit.into();
        let target = if let Some(p) = m.percent {
            Target::Percent(p)
        } else if let Some(n) = m.long_edge {
            Target::LongEdge(n)
        } else if let Some(n) = m.short_edge {
            Target::ShortEdge(n)
        } else if let Some(size) = m.box_size {
            Target::Box { size, fit }
        } else if let Some(width) = m.print_width {
            Target::Print {
                width,
                height: self.print_height.expect("clap requires it"),
                dpi: self.dpi.expect("clap requires it"),
                fit,
            }
        } else {
            Target::Megapixels(m.megapixels.expect("clap requires one mode"))
        };
        let options = resize::Options {
            never_enlarge: !self.allow_enlarge,
            round_to: self.round_to.max(1),
        };
        // Catch a bad value (such as 500%) before touching any file.
        resize::plan(Dimensions::new(1000, 1000), &target, &options).map_err(|e| e.to_string())?;
        Ok((target, options))
    }
}

/// Prints a batch the way the result toast shows it. Ok(false) if any file failed.
fn print_report(report: &Report) -> Result<bool, String> {
    for d in &report.done {
        let size = if d.from == d.to {
            String::new()
        } else {
            format!(" · {} → {} px", d.from, d.to)
        };
        println!(
            "{} → {} · {}{size}",
            d.input.display(),
            d.output.display(),
            describe_change(d.before, d.after)
        );
        if let Some(note) = &d.note {
            println!("  {note}");
        }
    }
    for f in &report.failed {
        println!("✗ {}", f.message);
    }
    if !report.skipped.is_empty() {
        match report.skipped.len() {
            1 => println!("Skipped 1 file that is not an image."),
            n => println!("Skipped {n} files that are not images."),
        }
    }
    if let Some(zip) = &report.zip {
        let bytes = fs::metadata(zip).map(|m| m.len()).unwrap_or(0);
        println!("ZIP: {} · {}", zip.display(), format_bytes(bytes));
    }
    let n = report.done.len();
    if n > 1 {
        println!(
            "{n} files · {}",
            describe_change(report.total_before(), report.total_after())
        );
    }
    Ok(report.failed.is_empty())
}

fn head(path: &Path) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    File::open(path)
        .and_then(|f| f.take(SNIFF_BYTES).read_to_end(&mut buf))
        .map_err(|e| open_error(path, e))?;
    Ok(buf)
}

fn read_text(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| open_error(path, e))
}

fn open_error(path: &Path, e: io::Error) -> String {
    pinwheel_engine::open_error(path, &e)
}

fn facts(path: &Path) -> Result<FileFacts, String> {
    let meta = fs::metadata(path).map_err(|e| open_error(path, e))?;
    if meta.is_dir() {
        return Err(format!(
            "{} is a folder. Pass the files inside it.",
            path.display()
        ));
    }
    let named = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let ext = sniff(&head(path)?)
        .map(str::to_string)
        .unwrap_or_else(|| canonical_ext(named));
    let dimensions = imagesize::size(path).ok().and_then(|s| {
        Some(Dimensions::new(
            s.width.try_into().ok()?,
            s.height.try_into().ok()?,
        ))
    });
    Ok(FileFacts {
        ext,
        bytes: meta.len(),
        dimensions,
    })
}

fn checksum(path: &Path, algo: Algo) -> Result<String, String> {
    fn hash<D: Digest + io::Write>(path: &Path) -> Result<String, String> {
        let mut file = File::open(path).map_err(|e| open_error(path, e))?;
        let mut d = D::new();
        io::copy(&mut file, &mut d).map_err(|e| open_error(path, e))?;
        Ok(d.finalize().iter().map(|b| format!("{b:02x}")).collect())
    }
    match algo {
        Algo::Md5 => hash::<Md5>(path),
        Algo::Sha1 => hash::<Sha1>(path),
        Algo::Sha256 => hash::<Sha256>(path),
    }
}
