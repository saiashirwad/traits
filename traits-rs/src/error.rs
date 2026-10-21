use serde::Serialize;

use crate::doc::SourceLocation;

/// Agent-facing tool failure. Serialized as JSON with a `_tag` discriminator,
/// matching the failure shapes of the TypeScript implementation.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "_tag")]
pub enum ToolFailure {
    AiError {
        message: String,
    },
    TraitNotFoundError {
        name: String,
        message: String,
    },
    TraitAlreadyExistsError {
        name: String,
        message: String,
    },
    TraitFilenameMismatchError {
        path: String,
        #[serde(rename = "expectedFilename")]
        expected_filename: String,
        message: String,
    },
    TraitParseError {
        path: String,
        message: String,
    },
    TraitAssessmentTooLargeError {
        #[serde(rename = "traitName")]
        trait_name: String,
        bytes: usize,
        #[serde(rename = "maxBytes")]
        max_bytes: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        location: Option<SourceLocation>,
        message: String,
    },
    GitFileListingError {
        directory: String,
        #[serde(rename = "exitCode")]
        exit_code: i32,
        message: String,
    },
    PlatformError {
        message: String,
    },
}

impl ToolFailure {
    pub fn ai(message: impl Into<String>) -> Self {
        Self::AiError { message: message.into() }
    }

    pub fn platform(message: impl Into<String>) -> Self {
        Self::PlatformError { message: message.into() }
    }

    pub fn not_found(name: &str) -> Self {
        Self::TraitNotFoundError {
            name: name.to_string(),
            message: format!("Trait {name} was not found"),
        }
    }

    pub fn already_exists(name: &str) -> Self {
        Self::TraitAlreadyExistsError {
            name: name.to_string(),
            message: format!("Trait {name} already exists"),
        }
    }

    pub fn filename_mismatch(path: &str, expected_filename: &str) -> Self {
        Self::TraitFilenameMismatchError {
            path: path.to_string(),
            expected_filename: expected_filename.to_string(),
            message: format!("Trait filename must be {expected_filename}, received {path}"),
        }
    }

    pub fn parse(path: &str, message: String) -> Self {
        Self::TraitParseError { path: path.to_string(), message }
    }

    pub fn too_large(trait_name: &str, bytes: usize, max_bytes: usize, location: Option<SourceLocation>) -> Self {
        Self::TraitAssessmentTooLargeError {
            trait_name: trait_name.to_string(),
            bytes,
            max_bytes,
            location,
            message: format!(
                "Source and trait {trait_name} exceed the decision request budget; split the source or shorten the trait."
            ),
        }
    }

    pub fn git_listing(directory: &str, exit_code: i32) -> Self {
        Self::GitFileListingError {
            directory: directory.to_string(),
            exit_code,
            message: format!("git ls-files exited with code {exit_code}"),
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::AiError { message }
            | Self::TraitNotFoundError { message, .. }
            | Self::TraitAlreadyExistsError { message, .. }
            | Self::TraitFilenameMismatchError { message, .. }
            | Self::TraitParseError { message, .. }
            | Self::TraitAssessmentTooLargeError { message, .. }
            | Self::GitFileListingError { message, .. }
            | Self::PlatformError { message } => message,
        }
    }
}

impl std::fmt::Display for ToolFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for ToolFailure {}

/// Startup and configuration failures. These keep the MCP server connected and
/// are surfaced through the fallback tool handlers.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{0}")]
    Message(String),
}

impl ConfigError {
    pub fn new(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}
