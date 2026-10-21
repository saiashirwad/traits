use serde::{Deserialize, Serialize};
use saphyr::{LoadableYamlNode, Yaml};

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

/// Diagnosis shown when the decoded frontmatter does not match the installed schema.
const SCHEMA_DIAGNOSIS: &str = "The document does not match the installed Trait schema. This may be a legacy format or invalid field values; inspect it before deciding migration is appropriate.";

/// Diagnosis shown when the document itself is malformed (bad YAML or delimiters).
const MALFORMED_DIAGNOSIS: &str = "The document could not be parsed. Inspect the original diagnostic for malformed YAML or missing/unterminated frontmatter; do not assume this is a schema migration.";

/// Build the agent-facing parse failure message: the path, a diagnosis detail, and
/// the consent-based recovery instructions ported from `TraitStore.ts`.
fn parse_failure(path: &str, detail: &str, diagnosis: &str) -> crate::error::ToolFailure {
    let message = format!(
        "Unable to parse trait {path}.\n\
         {detail}\n\
         \n\
         {diagnosis}\n\
         Recovery requires developer consent. Ask the developer: \"May I automatically resolve the stored-trait parse errors and retry the blocked operation?\"\n\
         Do not change files before approval. Do not merely report the blocker and stop without offering this recovery choice. If declined, leave files unchanged and report the operation as incomplete.\n\
         After approval, inspect the entire affected trait store, not just the first failing file. Classify each failure and migrate repairable legacy formats to the installed Trait schema; repair malformed documents only where the intended content is clear.\n\
         Preserve names, authoritative requirements, applicability, exceptions, examples, and all other supporting context. If a repair is ambiguous or unsupported, ask a focused question rather than guessing, inventing intent, or dropping information. Leave already-valid traits unchanged.\n\
         The current format requires name, subject, requirement, and purpose in YAML frontmatter; applicability and guidance are optional text, and the Markdown body is optional context. Purpose explains intended benefit, not additional obligations. Legacy body requirements belong in requirement; when/mime/skip become applicability prose; diffs and additional information belong in context.\n\
         Validate all files in the affected store with the installed Trait schema and serialize format before retrying the originally blocked operation with its original arguments. A server restart is not necessary: repaired files are read on retry.\n\
         No automatic migration has occurred. The original operation failed; do not treat it as successful or claim a completed check until the retry succeeds."
    );
    crate::error::ToolFailure::parse(path, message)
}

/// Why frontmatter could not be split from the document body.
enum SplitError {
    Missing,
    Unterminated,
}

/// Split a document into its raw frontmatter text and its body.
///
/// The document must open with `---\n` or `---\r\n` at byte 0. The frontmatter
/// ends at the first column-0 line that is exactly `---` followed by `\n`,
/// `\r\n`, or end of input (in which case the body is empty).
fn split_frontmatter(text: &str) -> Result<(&str, &str), SplitError> {
    let rest = if let Some(rest) = text.strip_prefix("---\r\n") {
        rest
    } else if let Some(rest) = text.strip_prefix("---\n") {
        rest
    } else if text == "---" || text.starts_with("---\r\n") || text.starts_with("---\n") {
        return Err(SplitError::Unterminated);
    } else {
        return Err(SplitError::Missing);
    };

    let mut pos = 0;
    while pos <= rest.len() {
        let remainder = &rest[pos..];
        if let Some(after) = remainder.strip_prefix("---") {
            if after.is_empty() {
                return Ok((&rest[..pos], ""));
            }
            if let Some(body) = after.strip_prefix("\r\n") {
                return Ok((&rest[..pos], body));
            }
            if let Some(body) = after.strip_prefix('\n') {
                return Ok((&rest[..pos], body));
            }
        }
        match remainder.find('\n') {
            Some(index) => pos += index + 1,
            None => break,
        }
    }
    Err(SplitError::Unterminated)
}

/// Parse a stored trait document (YAML frontmatter plus optional Markdown body).
pub fn parse(text: &str, path: &str) -> Result<Trait, crate::error::ToolFailure> {
    let (yaml_text, body) = split_frontmatter(text).map_err(|error| {
        let detail = match error {
            SplitError::Missing => "MissingFrontmatterError: Document must start with YAML frontmatter",
            SplitError::Unterminated => "UnterminatedFrontmatterError: Frontmatter is missing its closing delimiter",
        };
        parse_failure(path, detail, MALFORMED_DIAGNOSIS)
    })?;

    let frontmatter = decode_frontmatter(yaml_text, path)?;

    let context = if body.is_empty() { None } else { Some(body.to_string()) };
    Ok(Trait { context, ..frontmatter })
}

/// Parse the raw frontmatter text as YAML and decode it into a `Trait`.
fn decode_frontmatter(yaml_text: &str, path: &str) -> Result<Trait, crate::error::ToolFailure> {
    let documents = Yaml::load_from_str(yaml_text).map_err(|error| {
        let marker = error.marker();
        let detail = format!(
            "InvalidYamlError: Invalid YAML frontmatter (line {}, column {})",
            marker.line(),
            marker.col()
        );
        parse_failure(path, &detail, MALFORMED_DIAGNOSIS)
    })?;

    if documents.len() > 1 {
        let detail = "InvalidYamlError: Invalid YAML frontmatter (multiple documents)";
        return Err(parse_failure(path, detail, MALFORMED_DIAGNOSIS));
    }

    let root = documents.first();
    let mapping = match root {
        Some(Yaml::Mapping(mapping)) => mapping,
        _ => return Err(schema_failure(path, "SchemaError: frontmatter must be a YAML mapping")),
    };

    let mut name: Option<String> = None;
    let mut subject: Option<String> = None;
    let mut requirement: Option<String> = None;
    let mut applicability: Option<String> = None;
    let mut purpose: Option<String> = None;
    let mut guidance: Option<String> = None;

    for (key, value) in mapping {
        let key = key
            .as_str()
            .ok_or_else(|| schema_failure(path, "SchemaError: frontmatter keys must be strings"))?;
        match key {
            "name" => name = Some(string_field(path, key, value)?),
            "subject" => subject = Some(string_field(path, key, value)?),
            "requirement" => requirement = Some(string_field(path, key, value)?),
            "applicability" => applicability = Some(string_field(path, key, value)?),
            "purpose" => purpose = Some(string_field(path, key, value)?),
            "guidance" => guidance = Some(string_field(path, key, value)?),
            other => {
                return Err(schema_failure(path, &format!("SchemaError: unknown frontmatter field {other}")));
            }
        }
    }

    let name = require_field(path, "name", name)?;
    let subject = require_field(path, "subject", subject)?;
    let requirement = require_field(path, "requirement", requirement)?;
    let purpose = require_field(path, "purpose", purpose)?;

    let name = TraitName::parse(&name)
        .map_err(|error| schema_failure(path, &format!("SchemaError: {error}")))?;

    Ok(Trait { name, subject, requirement, applicability, purpose, guidance, context: None })
}

fn schema_failure(path: &str, detail: &str) -> crate::error::ToolFailure {
    parse_failure(path, detail, SCHEMA_DIAGNOSIS)
}

/// Decode a required scalar field as a present, non-blank string.
fn string_field(path: &str, key: &str, value: &Yaml<'_>) -> Result<String, crate::error::ToolFailure> {
    let text = value
        .as_str()
        .ok_or_else(|| schema_failure(path, &format!("SchemaError: field {key} must be a string")))?;
    if !text.chars().any(|c| !c.is_whitespace()) {
        return Err(schema_failure(path, &format!("SchemaError: field {key} must not be blank")));
    }
    Ok(text.to_string())
}

/// Require a field to have been present in the frontmatter mapping.
fn require_field(
    path: &str,
    key: &str,
    value: Option<String>,
) -> Result<String, crate::error::ToolFailure> {
    value.ok_or_else(|| schema_failure(path, &format!("SchemaError: field {key} is required")))
}

/// Serialize a trait into the canonical stored format.
pub fn serialize(trait_value: &Trait) -> String {
    let mut frontmatter = String::new();
    emit_field(&mut frontmatter, "name", trait_value.name.as_str());
    emit_field(&mut frontmatter, "subject", &trait_value.subject);
    emit_field(&mut frontmatter, "requirement", &trait_value.requirement);
    if let Some(applicability) = &trait_value.applicability {
        emit_field(&mut frontmatter, "applicability", applicability);
    }
    emit_field(&mut frontmatter, "purpose", &trait_value.purpose);
    if let Some(guidance) = &trait_value.guidance {
        emit_field(&mut frontmatter, "guidance", guidance);
    }

    let body = trait_value.context.as_deref().unwrap_or("");
    format!("---\n{frontmatter}---\n{body}")
}

/// Emit one `key: value` frontmatter line, choosing a block, plain, or quoted scalar.
fn emit_field(out: &mut String, key: &str, value: &str) {
    if value.contains('\n') {
        emit_block_scalar(out, key, value);
    } else if plain_scalar_is_safe(value) {
        out.push_str(key);
        out.push_str(": ");
        out.push_str(value);
        out.push('\n');
    } else {
        out.push_str(key);
        out.push_str(": ");
        out.push_str(&double_quoted(value));
        out.push('\n');
    }
}

/// Emit a multi-line string as a YAML literal block with correct chomping and,
/// when needed, an explicit indentation indicator.
fn emit_block_scalar(out: &mut String, key: &str, value: &str) {
    let trailing = value.len() - value.trim_end_matches('\n').len();
    let core = &value[..value.len() - trailing];
    let lines: Vec<&str> = if core.is_empty() { vec![""] } else { core.split('\n').collect() };

    let needs_indent_indicator = lines
        .iter()
        .find(|line| !line.is_empty())
        .is_some_and(|line| line.starts_with(' ') || line.starts_with('\t'));

    let chomp = match trailing {
        0 => "-",
        1 => "",
        _ => "+",
    };

    out.push_str(key);
    out.push_str(": |");
    if needs_indent_indicator {
        out.push('2');
    }
    out.push_str(chomp);
    out.push('\n');

    for line in &lines {
        if line.is_empty() {
            out.push('\n');
        } else {
            out.push_str("  ");
            out.push_str(line);
            out.push('\n');
        }
    }

    for _ in 0..trailing.saturating_sub(1) {
        out.push('\n');
    }
}

/// Whether a single-line value can be emitted as a plain (unquoted) YAML scalar.
fn plain_scalar_is_safe(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    if value.starts_with(char::is_whitespace) || value.ends_with(char::is_whitespace) {
        return false;
    }
    if value.contains(": ") || value.contains(" #") || value.ends_with(':') {
        return false;
    }
    if value.chars().any(|c| c.is_control() || c == '\t' || c == '\r') {
        return false;
    }
    const LEADING: [char; 19] =
        ['-', '?', '*', '&', '!', '%', '@', '`', '|', '>', '#', '"', '\'', '[', ']', '{', '}', ',', ':'];
    if value.starts_with(&LEADING[..]) {
        return false;
    }
    if looks_like_timestamp(value) {
        return false;
    }
    // Final safety net: the plain scalar must resolve back to the identical string.
    matches!(Yaml::load_from_str(value), Ok(documents) if documents.len() == 1 && documents[0].as_str() == Some(value))
}

/// Whether a value looks like a YAML timestamp (`2001-12-14` or `2001-12-14T21:59:43`).
fn looks_like_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 10 {
        return false;
    }
    let digits = |slice: &[u8]| slice.iter().all(u8::is_ascii_digit);
    let date = digits(&bytes[0..4]) && bytes[4] == b'-' && digits(&bytes[5..7]) && bytes[7] == b'-' && digits(&bytes[8..10]);
    if !date {
        return false;
    }
    bytes.len() == 10 || matches!(bytes[10], b'T' | b't' | b' ')
}

/// Emit a double-quoted YAML scalar with escapes.
fn double_quoted(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
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
