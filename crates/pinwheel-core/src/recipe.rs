//! Recipes: several steps chained into one slice, stored as plain TOML so
//! they can be emailed, posted or shared.
//!
//! ```toml
//! name = "Blog image"
//! accepts = ["image"]
//!
//! [[step]]
//! tool = "resize"
//! long_edge = 1600
//! when = { wider_than = 2000 }
//!
//! [[step]]
//! tool = "convert"
//! to = "webp"
//! max_size = "300 KB"
//!
//! [output]
//! name = "{name}-blog"
//! ```

use serde::Deserialize;

use crate::filetype::{canonical_ext, family_of, Family};
use crate::naming::validate_template;
use crate::resize::Dimensions;
use crate::units::{SizeRange, SizeSpec};
use crate::Error;

/// Tool ids a step can name.
pub const TOOLS: &[&str] = &[
    "convert",
    "resize",
    "crop",
    "rotate",
    "compress",
    "adjust",
    "annotate",
    "redact",
    "background",
    "watermark",
    "remove-background",
    "scan-cleanup",
    "metadata",
    "rename",
    "merge",
    "split",
    "trim",
    "fade",
    "normalise",
    "mute",
    "extract-audio",
    "zip",
    "custom",
];

/// Settings that hold a file size and must parse as one.
const SIZE_SETTINGS: &[&str] = &["max_size", "min_size", "size"];

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub name: String,
    /// Family ids ("image", "pdf", …) or extensions ("heic"). Empty accepts anything.
    #[serde(default)]
    pub accepts: Vec<String>,
    #[serde(rename = "step", default)]
    pub steps: Vec<Step>,
    #[serde(default)]
    pub output: Output,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Step {
    pub tool: String,
    #[serde(default)]
    pub when: Option<Condition>,
    /// The tool's own settings, checked by the tool when it runs.
    #[serde(flatten)]
    pub settings: toml::Table,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Output {
    pub name: Option<String>,
}

/// When a step runs. Every condition given must hold.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub wider_than: Option<u32>,
    pub taller_than: Option<u32>,
    pub larger_than: Option<String>,
    /// A family id or extension: "pdf", "image", "heic".
    pub kind: Option<String>,
}

/// What a step condition can look at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFacts {
    pub ext: String,
    pub bytes: u64,
    pub dimensions: Option<Dimensions>,
}

impl FileFacts {
    fn is(&self, kind: &str) -> bool {
        match Family::from_id(kind) {
            Some(family) => family_of(&self.ext) == Some(family),
            None => canonical_ext(&self.ext) == canonical_ext(kind),
        }
    }
}

impl Condition {
    pub fn matches(&self, file: &FileFacts) -> bool {
        let w = file.dimensions.map_or(0, |d| d.width);
        let h = file.dimensions.map_or(0, |d| d.height);
        self.wider_than.is_none_or(|min| w > min)
            && self.taller_than.is_none_or(|min| h > min)
            && self.larger_than.as_deref().is_none_or(|s| {
                s.parse::<SizeSpec>()
                    .is_ok_and(|s| file.bytes > s.max_bytes())
            })
            && self.kind.as_deref().is_none_or(|k| file.is(k))
    }
}

impl Recipe {
    pub fn from_toml(text: &str) -> Result<Recipe, Error> {
        let recipe: Recipe =
            toml::from_str(text).map_err(|e| Error::Recipe(e.message().to_string()))?;
        recipe.validate()?;
        Ok(recipe)
    }

    fn validate(&self) -> Result<(), Error> {
        if self.name.trim().is_empty() {
            return Err(Error::Recipe("it needs a name.".into()));
        }
        if self.steps.is_empty() {
            return Err(Error::Recipe(
                "it has no steps. Add at least one [[step]].".into(),
            ));
        }
        for kind in &self.accepts {
            if Family::from_id(kind).is_none() && family_of(kind).is_none() {
                return Err(Error::Recipe(format!(
                    "it accepts \"{kind}\", which is not a file type Pinwheel knows."
                )));
            }
        }
        for (i, step) in self.steps.iter().enumerate() {
            let n = i + 1;
            if !TOOLS.contains(&step.tool.as_str()) {
                return Err(Error::Recipe(format!(
                    "step {n} uses \"{}\", which is not a tool.",
                    step.tool
                )));
            }
            for key in SIZE_SETTINGS {
                if let Some(v) = step.settings.get(*key) {
                    let ok = v.as_str().is_some_and(|s| s.parse::<SizeRange>().is_ok());
                    if !ok {
                        return Err(Error::Recipe(format!(
                            "step {n} has {key} = {v}, which is not a file size."
                        )));
                    }
                }
            }
            if let Some(size) = step.when.as_ref().and_then(|w| w.larger_than.as_deref()) {
                size.parse::<SizeSpec>()
                    .map_err(|e| Error::Recipe(format!("step {n}: {e}")))?;
            }
            if step.tool == "convert" && !step.settings.contains_key("to") {
                return Err(Error::Recipe(format!(
                    "step {n} converts but does not say to what. Add to = \"jpg\"."
                )));
            }
            if step.tool == "custom" && !step.settings.get("command").is_some_and(|c| c.is_str()) {
                return Err(Error::Recipe(format!(
                    "step {n} is a custom action without a command."
                )));
            }
        }
        if let Some(template) = &self.output.name {
            validate_template(template).map_err(|e| Error::Recipe(e.to_string()))?;
        }
        Ok(())
    }

    /// Recipes with a custom action are labelled "Runs a program" so nobody
    /// runs a command by accident.
    pub fn runs_program(&self) -> bool {
        self.steps.iter().any(|s| s.tool == "custom")
    }

    pub fn accepts_file(&self, file: &FileFacts) -> bool {
        self.accepts.is_empty() || self.accepts.iter().any(|k| file.is(k))
    }

    /// The steps that apply to `file`, in order.
    pub fn steps_for<'a>(&'a self, file: &'a FileFacts) -> impl Iterator<Item = &'a Step> {
        self.steps
            .iter()
            .filter(|s| s.when.as_ref().is_none_or(|c| c.matches(file)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOG: &str = r#"
name = "Blog image"
accepts = ["image"]

[[step]]
tool = "resize"
long_edge = 1600
never_enlarge = true
when = { wider_than = 2000 }

[[step]]
tool = "metadata"
remove = "location"

[[step]]
tool = "convert"
to = "webp"
max_size = "300 KB"

[output]
name = "{name}-blog"
"#;

    #[test]
    fn parses_the_vision_example() {
        let r = Recipe::from_toml(BLOG).unwrap();
        assert_eq!(r.name, "Blog image");
        assert_eq!(r.steps.len(), 3);
        assert_eq!(r.steps[0].settings["long_edge"].as_integer(), Some(1600));
        assert_eq!(r.output.name.as_deref(), Some("{name}-blog"));
        assert!(!r.runs_program());
    }

    #[test]
    fn conditions_pick_steps() {
        let r = Recipe::from_toml(BLOG).unwrap();
        let big = FileFacts {
            ext: "heic".into(),
            bytes: 3_000_000,
            dimensions: Some(Dimensions::new(4032, 3024)),
        };
        let small = FileFacts {
            dimensions: Some(Dimensions::new(1200, 900)),
            ..big.clone()
        };
        assert_eq!(r.steps_for(&big).count(), 3);
        assert_eq!(
            r.steps_for(&small)
                .map(|s| s.tool.as_str())
                .collect::<Vec<_>>(),
            ["metadata", "convert"]
        );
        assert!(r.accepts_file(&big));
        assert!(!r.accepts_file(&FileFacts {
            ext: "pdf".into(),
            bytes: 1,
            dimensions: None
        }));

        let c = Condition {
            larger_than: Some("1 MB".into()),
            kind: Some("pdf".into()),
            ..Condition::default()
        };
        assert!(c.matches(&FileFacts {
            ext: "pdf".into(),
            bytes: 2_000_000,
            dimensions: None
        }));
        assert!(!c.matches(&FileFacts {
            ext: "pdf".into(),
            bytes: 500_000,
            dimensions: None
        }));
    }

    #[test]
    fn plain_errors() {
        let err = |t: &str| Recipe::from_toml(t).unwrap_err().to_string();
        assert!(err("name = \"x\"").contains("no steps"));
        assert!(err("name = \"x\"\n[[step]]\ntool = \"teleport\"").contains("not a tool"));
        assert!(err("name = \"x\"\n[[step]]\ntool = \"convert\"").contains("to = \"jpg\""));
        assert!(err(
            "name = \"x\"\n[[step]]\ntool = \"convert\"\nto = \"jpg\"\nmax_size = \"big\""
        )
        .contains("not a file size"));
        assert!(
            err("name = \"x\"\naccepts = [\"exe\"]\n[[step]]\ntool = \"zip\"")
                .contains("not a file type")
        );
        assert!(
            err("name = \"x\"\n[[step]]\ntool = \"zip\"\n[output]\nname = \"{nope}\"")
                .contains("not a token")
        );
    }

    #[test]
    fn custom_actions_are_labelled() {
        let r = Recipe::from_toml("name = \"x\"\n[[step]]\ntool = \"custom\"\ncommand = \"optipng {input} -out {output}\"").unwrap();
        assert!(r.runs_program());
    }
}
