//! Output names: templates with tokens, and never overwriting anything.
//!
//! A template names the file, for example `{name}-blog`. The output's
//! extension is added unless the template places `{ext}` itself.

use std::path::{Path, PathBuf};

use crate::Error;

pub const TOKENS: [&str; 9] = [
    "name",
    "action",
    "date",
    "photo-date",
    "width",
    "height",
    "n",
    "preset",
    "ext",
];

/// The default template: `{name}`, or `{name}-{action}` when the format stays
/// the same, so the result never collides with the original.
pub fn default_template(same_format: bool) -> &'static str {
    if same_format {
        "{name}-{action}"
    } else {
        "{name}"
    }
}

/// Values for the tokens. A token the template uses must be set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Context {
    pub name: String,
    pub ext: String,
    pub action: Option<String>,
    pub date: Option<String>,
    pub photo_date: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub n: Option<u32>,
    pub preset: Option<String>,
}

impl Context {
    fn get(&self, token: &str) -> Option<String> {
        match token {
            "name" => Some(self.name.clone()),
            "ext" => Some(self.ext.clone()),
            "action" => self.action.clone(),
            "date" => self.date.clone(),
            "photo-date" => self.photo_date.clone(),
            "width" => self.width.map(|v| v.to_string()),
            "height" => self.height.map(|v| v.to_string()),
            "n" => self.n.map(|v| v.to_string()),
            "preset" => self.preset.clone(),
            _ => None,
        }
    }
}

enum Piece<'a> {
    Text(&'a str),
    Token(&'a str),
}

fn parse(template: &str) -> Result<Vec<Piece<'_>>, Error> {
    let mut pieces = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        if open > 0 {
            pieces.push(Piece::Text(&rest[..open]));
        }
        let close = rest[open..].find('}').ok_or_else(|| {
            Error::Template(format!("\"{template}\" has a {{ without a matching }}."))
        })?;
        let token = &rest[open + 1..open + close];
        if !TOKENS.contains(&token) {
            return Err(Error::Template(format!(
                "{{{token}}} is not a token. Use one of: {}.",
                TOKENS.map(|t| format!("{{{t}}}")).join(" ")
            )));
        }
        pieces.push(Piece::Token(token));
        rest = &rest[open + close + 1..];
    }
    if rest.contains('}') {
        return Err(Error::Template(format!(
            "\"{template}\" has a }} without a matching {{."
        )));
    }
    if !rest.is_empty() {
        pieces.push(Piece::Text(rest));
    }
    Ok(pieces)
}

pub fn validate_template(template: &str) -> Result<(), Error> {
    let pieces = parse(template)?;
    if pieces.is_empty() {
        return Err(Error::Template("the template is empty.".into()));
    }
    Ok(())
}

/// Characters no system allows in a file name, or that would change folder.
fn clean(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect()
}

/// Renders a template into a file name, extension included.
pub fn render(template: &str, ctx: &Context) -> Result<String, Error> {
    let mut out = String::new();
    let mut has_ext = false;
    for piece in parse(template)? {
        match piece {
            Piece::Text(t) => out.push_str(&clean(t)),
            Piece::Token(t) => {
                has_ext |= t == "ext";
                let value = ctx.get(t).ok_or_else(|| {
                    Error::Template(format!("{{{t}}} has no value for this file."))
                })?;
                out.push_str(&clean(&value));
            }
        }
    }
    // Windows drops trailing dots and spaces, so never rely on them.
    let mut out = out.trim_end_matches([' ', '.']).to_string();
    if out.is_empty() {
        return Err(Error::Template(format!(
            "\"{template}\" gives an empty name."
        )));
    }
    if !has_ext && !ctx.ext.is_empty() {
        out.push('.');
        out.push_str(&ctx.ext);
    }
    Ok(out)
}

/// The first free path for `file_name` in `dir`: `photo.jpg`, then
/// `photo (2).jpg`, `photo (3).jpg` and so on. Nothing is ever overwritten.
pub fn unique_path(dir: &Path, file_name: &str, exists: impl Fn(&Path) -> bool) -> PathBuf {
    let candidate = dir.join(file_name);
    if !exists(&candidate) {
        return candidate;
    }
    let as_path = Path::new(file_name);
    let stem = as_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_name);
    let ext = as_path.extension().and_then(|s| s.to_str());
    (2u32..)
        .map(|n| {
            dir.join(match ext {
                Some(ext) => format!("{stem} ({n}).{ext}"),
                None => format!("{stem} ({n})"),
            })
        })
        .find(|p| !exists(p))
        .expect("an unused number exists")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Context {
        Context {
            name: "photo".into(),
            ext: "webp".into(),
            action: Some("resize".into()),
            width: Some(1600),
            height: Some(1200),
            ..Context::default()
        }
    }

    #[test]
    fn renders_templates() {
        assert_eq!(render("{name}-blog", &ctx()).unwrap(), "photo-blog.webp");
        assert_eq!(
            render(default_template(false), &ctx()).unwrap(),
            "photo.webp"
        );
        assert_eq!(
            render(default_template(true), &ctx()).unwrap(),
            "photo-resize.webp"
        );
        assert_eq!(
            render("{name}_{width}x{height}", &ctx()).unwrap(),
            "photo_1600x1200.webp"
        );
        assert_eq!(render("{name}.{ext}", &ctx()).unwrap(), "photo.webp");
    }

    #[test]
    fn rejects_bad_templates() {
        assert!(validate_template("{nme}").is_err());
        assert!(validate_template("{name").is_err());
        assert!(validate_template("name}").is_err());
        assert!(validate_template("").is_err());
        assert!(render("{preset}", &ctx()).is_err(), "unset token");
    }

    #[test]
    fn values_cannot_escape_the_folder() {
        let mut c = ctx();
        c.name = "../../etc/passwd".into();
        assert_eq!(render("{name}", &c).unwrap(), ".._.._etc_passwd.webp");
    }

    #[test]
    fn never_overwrites() {
        let taken = ["out/photo.jpg", "out/photo (2).jpg"];
        let exists = |p: &Path| taken.iter().any(|t| Path::new(t) == p);
        assert_eq!(
            unique_path(Path::new("out"), "photo.jpg", exists),
            Path::new("out/photo (3).jpg")
        );
        assert_eq!(
            unique_path(Path::new("out"), "other.jpg", exists),
            Path::new("out/other.jpg")
        );
    }
}
