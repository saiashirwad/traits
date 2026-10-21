use serde::{Deserialize, Serialize};

/// Unique trait identity: descriptive kebab-case, no redundant `trait-` prefix,
/// at most 80 ASCII characters. Also used as the Markdown filename stem.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TraitName(pub String);

#[derive(Debug, thiserror::Error, PartialEq)]
#[error("Invalid trait name: {0}")]
pub struct TraitNameError(pub String);

impl TraitName {
    pub fn parse(value: &str) -> Result<Self, TraitNameError> {
        let valid = !value.is_empty()
            && value.len() <= 80
            && !value.starts_with("trait-")
            && value
                .split('-')
                .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()));
        if valid { Ok(Self(value.to_string())) } else { Err(TraitNameError(value.to_string())) }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A complete stored trait. `context` is the optional Markdown body and is not
/// part of the YAML frontmatter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trait {
    pub name: TraitName,
    pub subject: String,
    pub requirement: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applicability: Option<String>,
    pub purpose: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guidance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

/// Parse a stored trait document (YAML frontmatter plus optional Markdown body).
pub fn parse(text: &str, path: &str) -> Result<Trait, crate::error::ToolFailure> {
    let _ = (text, path);
    todo!("delegate: docstore")
}

/// Serialize a trait into the canonical stored format.
pub fn serialize(trait_value: &Trait) -> String {
    let _ = trait_value;
    todo!("delegate: docstore")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_tag")]
pub enum SourceLocation {
    #[serde(rename = "File")]
    File {
        path: String,
        #[serde(rename = "startLine")]
        start_line: u32,
    },
    #[serde(rename = "Generated")]
    Generated {},
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<SourceLocation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Classification {
    Compliant,
    Violation,
    NotApplicable,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Probabilities {
    pub compliant: f64,
    pub violation: f64,
    #[serde(rename = "not-applicable")]
    pub not_applicable: f64,
    pub inconclusive: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assessment {
    pub classification: Classification,
    pub probabilities: Probabilities,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraitEvaluation {
    pub r#trait: Trait,
    pub source: Source,
    pub assessment: Assessment,
}

impl TraitEvaluation {
    pub fn requires_review(&self) -> bool {
        matches!(
            self.assessment.classification,
            Classification::Violation | Classification::Inconclusive
        )
    }
}
