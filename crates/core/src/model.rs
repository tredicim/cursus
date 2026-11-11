//! The data model shared by the CLI, the renderer and the future desktop app.
//!
//! Entries and targets are plain YAML documents. The Rust types are the source
//! of truth: the JSON Schema, the validation rules and the renderer all derive
//! from them.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};

/// A year and month, serialized as `YYYY-MM`.
///
/// Only month precision is needed for a CV, and a dedicated type keeps invalid
/// dates out of the model: `2025-13` fails while the YAML is parsed, with the
/// field path reported by the loader.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct YearMonth(String);

impl YearMonth {
    /// Parse `YYYY-MM`, rejecting impossible months.
    pub fn parse(value: &str) -> Result<Self, String> {
        let bytes = value.as_bytes();
        let well_formed = bytes.len() == 7
            && bytes[4] == b'-'
            && bytes[..4].iter().all(u8::is_ascii_digit)
            && bytes[5..].iter().all(u8::is_ascii_digit);

        if !well_formed {
            return Err(format!("expected a date in YYYY-MM format, got `{value}`"));
        }

        let month: u8 = value[5..7]
            .parse()
            .map_err(|_| format!("expected a date in YYYY-MM format, got `{value}`"))?;
        if !(1..=12).contains(&month) {
            return Err(format!("month must be between 01 and 12, got `{value}`"));
        }

        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for YearMonth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for YearMonth {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::parse(&raw).map_err(serde::de::Error::custom)
    }
}

impl JsonSchema for YearMonth {
    fn schema_name() -> Cow<'static, str> {
        "YearMonth".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "string",
            "pattern": "^[0-9]{4}-(0[1-9]|1[0-2])$",
            "examples": ["2025-06"],
            "description": "A year and month in YYYY-MM format."
        })
    }
}

/// What an entry represents.
///
/// Adding a kind means adding one variant here and a label below; validation
/// and rendering pick the new kind up without another match.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum EntryKind {
    Experience,
    Project,
    Education,
    Skill,
    Award,
    Certification,
    Other,
}

impl EntryKind {
    /// Human-readable section heading used by templates.
    pub fn label(self) -> &'static str {
        match self {
            Self::Experience => "Experience",
            Self::Project => "Projects",
            Self::Education => "Education",
            Self::Skill => "Skills",
            Self::Award => "Awards",
            Self::Certification => "Certifications",
            Self::Other => "Other",
        }
    }
}

impl fmt::Display for EntryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// One framing of an entry, e.g. the same internship written for a backend role
/// and for a data-science role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Variant {
    /// Optional prose paragraph shown before the bullets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// The bullet points for this framing. At least one bullet or a summary is
    /// required by validation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bullets: Vec<String>,
}

/// A link attached to an entry and shown by templates that support links.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub label: String,
    pub url: String,
}

/// One item in the career library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Stable identifier, unique across all entries.
    pub id: String,
    pub kind: EntryKind,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<YearMonth>,
    /// Omit for an ongoing entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<YearMonth>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
    /// Named framings, keyed by variant name. Must not be empty.
    pub variants: BTreeMap<String, Variant>,
}

/// The LaTeX theme used to render a target.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Template {
    /// Clean, single-column, ATS-friendly layout.
    Classic,
}

impl Template {
    pub fn id(self) -> &'static str {
        match self {
            Self::Classic => "classic",
        }
    }
}

impl fmt::Display for Template {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// One entry and the variant to use for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TargetInclude {
    /// ID of an entry in `data/`.
    pub id: String,
    /// Variant of that entry. Defaults to `default`.
    #[serde(default = "default_variant")]
    pub variant: String,
}

fn default_variant() -> String {
    "default".to_owned()
}

/// One CV to build: the ordered selection of entries and the template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Target {
    /// Display name, also used as the document title.
    pub name: String,
    pub template: Template,
    /// Entries to include. Must not be empty.
    pub include: Vec<TargetInclude>,
}

/// Any top-level Cursus document.
///
/// This exists so `cursus schema` can publish one schema covering both file
/// types. The loader still parses entries and targets separately, which keeps
/// error messages precise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Document {
    Entry(Entry),
    Target(Target),
}
