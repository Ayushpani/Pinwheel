//! What a file really is, read from its first bytes rather than its name.
//! This powers "Fix wrong extension" and picks which rings the wheel shows.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Family {
    Image,
    Vector,
    Pdf,
    Document,
    Spreadsheet,
    Data,
    Presentation,
    Audio,
    Video,
    Subtitle,
    Archive,
    Font,
    Contact,
}

impl Family {
    pub const ALL: [Family; 13] = [
        Family::Image,
        Family::Vector,
        Family::Pdf,
        Family::Document,
        Family::Spreadsheet,
        Family::Data,
        Family::Presentation,
        Family::Audio,
        Family::Video,
        Family::Subtitle,
        Family::Archive,
        Family::Font,
        Family::Contact,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Family::Image => "image",
            Family::Vector => "vector",
            Family::Pdf => "pdf",
            Family::Document => "document",
            Family::Spreadsheet => "spreadsheet",
            Family::Data => "data",
            Family::Presentation => "presentation",
            Family::Audio => "audio",
            Family::Video => "video",
            Family::Subtitle => "subtitle",
            Family::Archive => "archive",
            Family::Font => "font",
            Family::Contact => "contact",
        }
    }

    pub fn from_id(id: &str) -> Option<Family> {
        Family::ALL.into_iter().find(|f| f.id() == id)
    }
}

impl fmt::Display for Family {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// Every extension Pinwheel knows, with its family.
const KINDS: &[(&str, Family)] = &[
    ("jpg", Family::Image),
    ("png", Family::Image),
    ("webp", Family::Image),
    ("avif", Family::Image),
    ("heic", Family::Image),
    ("jxl", Family::Image),
    ("tiff", Family::Image),
    ("bmp", Family::Image),
    ("gif", Family::Image),
    ("ico", Family::Image),
    ("icns", Family::Image),
    ("tga", Family::Image),
    ("psd", Family::Image),
    ("cr2", Family::Image),
    ("cr3", Family::Image),
    ("nef", Family::Image),
    ("arw", Family::Image),
    ("dng", Family::Image),
    ("raf", Family::Image),
    ("orf", Family::Image),
    ("rw2", Family::Image),
    ("svg", Family::Vector),
    ("pdf", Family::Pdf),
    ("docx", Family::Document),
    ("odt", Family::Document),
    ("rtf", Family::Document),
    ("txt", Family::Document),
    ("md", Family::Document),
    ("html", Family::Document),
    ("epub", Family::Document),
    ("xlsx", Family::Spreadsheet),
    ("xls", Family::Spreadsheet),
    ("ods", Family::Spreadsheet),
    ("csv", Family::Data),
    ("tsv", Family::Data),
    ("json", Family::Data),
    ("yaml", Family::Data),
    ("xml", Family::Data),
    ("toml", Family::Data),
    ("pptx", Family::Presentation),
    ("odp", Family::Presentation),
    ("mp3", Family::Audio),
    ("m4a", Family::Audio),
    ("aac", Family::Audio),
    ("wav", Family::Audio),
    ("flac", Family::Audio),
    ("ogg", Family::Audio),
    ("opus", Family::Audio),
    ("aiff", Family::Audio),
    ("wma", Family::Audio),
    ("amr", Family::Audio),
    ("m4r", Family::Audio),
    ("mp4", Family::Video),
    ("mov", Family::Video),
    ("mkv", Family::Video),
    ("webm", Family::Video),
    ("avi", Family::Video),
    ("wmv", Family::Video),
    ("flv", Family::Video),
    ("mts", Family::Video),
    ("3gp", Family::Video),
    ("srt", Family::Subtitle),
    ("vtt", Family::Subtitle),
    ("ass", Family::Subtitle),
    ("zip", Family::Archive),
    ("7z", Family::Archive),
    ("tar", Family::Archive),
    ("gz", Family::Archive),
    ("xz", Family::Archive),
    ("rar", Family::Archive),
    ("iso", Family::Archive),
    ("ttf", Family::Font),
    ("otf", Family::Font),
    ("woff", Family::Font),
    ("woff2", Family::Font),
    ("vcf", Family::Contact),
    ("ics", Family::Contact),
];

/// The one spelling Pinwheel uses for an extension: "JPEG" → "jpg".
pub fn canonical_ext(ext: &str) -> String {
    let lower = ext.trim_start_matches('.').to_ascii_lowercase();
    match lower.as_str() {
        "jpeg" | "jpe" | "jfif" => "jpg",
        "tif" => "tiff",
        "heif" => "heic",
        "m4v" => "mp4",
        "htm" => "html",
        "markdown" => "md",
        "yml" => "yaml",
        "m2ts" => "mts",
        "aif" => "aiff",
        "ssa" => "ass",
        "tgz" => "gz",
        "qt" => "mov",
        _ => return lower,
    }
    .to_string()
}

pub fn family_of(ext: &str) -> Option<Family> {
    let ext = canonical_ext(ext);
    KINDS.iter().find(|(e, _)| *e == ext).map(|(_, f)| *f)
}

fn at(bytes: &[u8], offset: usize, magic: &[u8]) -> bool {
    bytes.get(offset..offset + magic.len()) == Some(magic)
}

fn contains(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.windows(needle.len()).any(|w| w == needle)
}

/// Reads the real type from the start of a file. Pass at least the first
/// 64 KB for best results. Returns the canonical extension, or `None` when
/// the bytes do not identify a type (plain text, for example).
pub fn sniff(b: &[u8]) -> Option<&'static str> {
    let ext = if at(b, 0, b"\xFF\xD8\xFF") {
        "jpg"
    } else if at(b, 0, b"\x89PNG\r\n\x1a\n") {
        "png"
    } else if at(b, 0, b"GIF87a") || at(b, 0, b"GIF89a") {
        "gif"
    } else if at(b, 0, b"RIFF") {
        match b.get(8..12)? {
            b"WEBP" => "webp",
            b"WAVE" => "wav",
            b"AVI " => "avi",
            _ => return None,
        }
    } else if at(b, 4, b"ftyp") {
        match b.get(8..12)? {
            b"avif" | b"avis" => "avif",
            b"heic" | b"heix" | b"heim" | b"heis" | b"hevc" | b"mif1" | b"msf1" => "heic",
            b"crx " => "cr3",
            b"M4A " | b"M4B " => "m4a",
            b"qt  " => "mov",
            b"3gp4" | b"3gp5" | b"3gp6" | b"3g2a" => "3gp",
            _ => "mp4",
        }
    } else if at(b, 0, b"\xFF\x0A") || at(b, 0, b"\x00\x00\x00\x0CJXL \r\n\x87\n") {
        "jxl"
    } else if at(b, 0, b"II*\x00") && at(b, 8, b"CR") {
        "cr2"
    } else if at(b, 0, b"IIRO") {
        "orf"
    } else if at(b, 0, b"IIU\x00") {
        "rw2"
    } else if at(b, 0, b"FUJIFILMCCD-RAW") {
        "raf"
    } else if at(b, 0, b"II*\x00") || at(b, 0, b"MM\x00*") {
        "tiff"
    } else if at(b, 0, b"BM") && b.len() >= 26 {
        "bmp"
    } else if at(b, 0, b"\x00\x00\x01\x00") {
        "ico"
    } else if at(b, 0, b"icns") {
        "icns"
    } else if at(b, 0, b"8BPS") {
        "psd"
    } else if at(b, 0, b"%PDF-") {
        "pdf"
    } else if at(b, 0, b"PK\x03\x04") || at(b, 0, b"PK\x05\x06") {
        zip_kind(b)
    } else if at(b, 0, b"7z\xBC\xAF\x27\x1C") {
        "7z"
    } else if at(b, 0, b"Rar!\x1A\x07") {
        "rar"
    } else if at(b, 0, b"\x1F\x8B") {
        "gz"
    } else if at(b, 0, b"\xFD7zXZ\x00") {
        "xz"
    } else if at(b, 257, b"ustar") {
        "tar"
    } else if at(b, 32769, b"CD001") {
        "iso"
    } else if at(b, 0, b"ID3") {
        "mp3"
    } else if b.len() >= 2 && b[0] == 0xFF && b[1] & 0xE0 == 0xE0 {
        // MPEG audio frame sync; layer bits 00 mean an ADTS AAC stream.
        if (b[1] >> 1) & 0b11 == 0 {
            "aac"
        } else {
            "mp3"
        }
    } else if at(b, 0, b"fLaC") {
        "flac"
    } else if at(b, 0, b"OggS") {
        if at(b, 28, b"OpusHead") {
            "opus"
        } else {
            "ogg"
        }
    } else if at(b, 0, b"FORM") && (at(b, 8, b"AIFF") || at(b, 8, b"AIFC")) {
        "aiff"
    } else if at(b, 0, b"#!AMR") {
        "amr"
    } else if at(b, 0, b"\x30\x26\xB2\x75\x8E\x66\xCF\x11") {
        "wmv"
    } else if at(b, 0, b"\x1A\x45\xDF\xA3") {
        if contains(&b[..b.len().min(64)], b"webm") {
            "webm"
        } else {
            "mkv"
        }
    } else if at(b, 0, b"FLV\x01") {
        "flv"
    } else if b.first() == Some(&0x47) && b.get(188) == Some(&0x47) {
        "mts"
    } else if at(b, 0, b"wOFF") {
        "woff"
    } else if at(b, 0, b"wOF2") {
        "woff2"
    } else if at(b, 0, b"\x00\x01\x00\x00\x00") {
        "ttf"
    } else if at(b, 0, b"OTTO") {
        "otf"
    } else if at(b, 0, b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1") {
        // Old Office container. Only Excel 97 is a format Pinwheel reads.
        if contains(b, b"W\x00o\x00r\x00k\x00b\x00o\x00o\x00k") {
            "xls"
        } else {
            return None;
        }
    } else {
        return text_kind(b);
    };
    Some(ext)
}

fn zip_kind(b: &[u8]) -> &'static str {
    // ODF and EPUB store their type as the first, uncompressed entry.
    if at(b, 30, b"mimetype") {
        let rest = &b[38..b.len().min(38 + 64)];
        for (mime, ext) in [
            (&b"application/epub+zip"[..], "epub"),
            (b"application/vnd.oasis.opendocument.text", "odt"),
            (b"application/vnd.oasis.opendocument.spreadsheet", "ods"),
            (b"application/vnd.oasis.opendocument.presentation", "odp"),
        ] {
            if rest.starts_with(mime) {
                return ext;
            }
        }
    }
    // Office Open XML names its parts by folder.
    if contains(b, b"word/") {
        "docx"
    } else if contains(b, b"xl/") {
        "xlsx"
    } else if contains(b, b"ppt/") {
        "pptx"
    } else {
        "zip"
    }
}

fn text_kind(b: &[u8]) -> Option<&'static str> {
    let head = std::str::from_utf8(&b[..b.len().min(4096)])
        .ok()
        .or_else(|| {
            // The cut may split a character; retry without the last few bytes.
            (1..4).find_map(|n| std::str::from_utf8(&b[..b.len().min(4096).saturating_sub(n)]).ok())
        })?;
    let t = head.trim_start_matches('\u{feff}').trim_start();
    let lower = t.chars().take(512).collect::<String>().to_ascii_lowercase();
    Some(if lower.starts_with("{\\rtf") {
        "rtf"
    } else if lower.starts_with("begin:vcard") {
        "vcf"
    } else if lower.starts_with("begin:vcalendar") {
        "ics"
    } else if lower.starts_with("webvtt") {
        "vtt"
    } else if lower.starts_with("[script info]") {
        "ass"
    } else if lower.starts_with("<!doctype html") || lower.starts_with("<html") {
        "html"
    } else if lower.contains("<svg") {
        "svg"
    } else {
        return None;
    })
}

/// When a file's name says one type but its bytes say another, the
/// extension it should have. `None` when it is already right or unknown.
pub fn wrong_extension(name_ext: &str, bytes: &[u8]) -> Option<&'static str> {
    let real = sniff(bytes)?;
    let named = canonical_ext(name_ext);
    // Formats built on another one: their sniffed base is not a mismatch.
    let compatible = matches!(
        (real, named.as_str()),
        ("tiff", "nef" | "arw" | "dng")
            | ("mp4", "m4a" | "m4r" | "mov")
            | ("m4a", "m4r" | "mp4")
            | (
                "zip",
                "docx" | "xlsx" | "pptx" | "epub" | "odt" | "ods" | "odp"
            )
            | ("ogg", "opus")
            | ("gz", "tar")
            | ("mkv", "webm")
            | ("ttf", "otf")
    );
    (real != named && !compatible).then_some(real)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_common_types() {
        let mut heic = b"\x00\x00\x00\x18ftypheic".to_vec();
        heic.extend([0; 8]);
        for (bytes, ext) in [
            (&b"\xFF\xD8\xFF\xE0\x00\x10JFIF"[..], "jpg"),
            (b"\x89PNG\r\n\x1a\n\x00\x00", "png"),
            (b"GIF89a....", "gif"),
            (b"RIFF\x00\x00\x00\x00WEBPVP8 ", "webp"),
            (b"RIFF\x00\x00\x00\x00WAVEfmt ", "wav"),
            (&heic, "heic"),
            (b"%PDF-1.7\n", "pdf"),
            (b"ID3\x04\x00", "mp3"),
            (b"fLaC\x00", "flac"),
            (b"wOF2\x00", "woff2"),
            (b"\xEF\xBB\xBF<?xml version=\"1.0\"?>\n<svg xmlns=", "svg"),
            (b"BEGIN:VCARD\r\nVERSION:3.0", "vcf"),
            (b"WEBVTT\n\n00:00.000", "vtt"),
        ] {
            assert_eq!(sniff(bytes), Some(ext), "{ext}");
        }
        assert_eq!(sniff(b"just some notes"), None);
        assert_eq!(sniff(b""), None);
    }

    #[test]
    fn tells_zip_based_formats_apart() {
        let mut epub = b"PK\x03\x04".to_vec();
        epub.resize(30, 0);
        epub.extend(b"mimetypeapplication/epub+zip");
        assert_eq!(sniff(&epub), Some("epub"));

        let mut docx = b"PK\x03\x04".to_vec();
        docx.resize(30, 0);
        docx.extend(b"[Content_Types].xml....word/document.xml");
        assert_eq!(sniff(&docx), Some("docx"));
    }

    #[test]
    fn finds_wrong_extensions() {
        let png = b"\x89PNG\r\n\x1a\n\x00\x00";
        assert_eq!(wrong_extension("jpg", png), Some("png"));
        assert_eq!(wrong_extension("PNG", png), None);
        assert_eq!(wrong_extension("jpeg", b"\xFF\xD8\xFF\xE0"), None);
        assert_eq!(wrong_extension("dng", b"II*\x00\x08\x00\x00\x00"), None);
        assert_eq!(wrong_extension("txt", b"hello"), None);
    }

    #[test]
    fn families() {
        assert_eq!(family_of("JPEG"), Some(Family::Image));
        assert_eq!(family_of(".mkv"), Some(Family::Video));
        assert_eq!(family_of("exe"), None);
        assert_eq!(Family::from_id("pdf"), Some(Family::Pdf));
    }
}
