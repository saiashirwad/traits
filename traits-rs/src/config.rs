use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JsonMode {
    /// Native `response_format: {type: "json_schema"}`.
    Schema,
    /// `response_format: {type: "json_object"}` with the schema described in the prompt.
    Object,
    /// No response_format; schema described in the prompt.
    Prompt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Typesafe,
    OpenCodeGo,
    DeepSeek,
    OpenRouter,
    OpenAi,
    Custom,
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub name: String,
    pub kind: ProviderKind,
    pub model: String,
    pub api_key_env: String,
    pub base_url: Option<String>,
    pub json_mode: JsonMode,
    pub reasoning_effort: Option<String>,
    pub timeout_ms: u64,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct OperationConfig {
    pub provider: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub config_root: PathBuf,
    pub store: PathBuf,
    pub concurrency: usize,
    pub exclude_gitignored: bool,
    pub excludes: Vec<String>,
    pub default_provider: Option<String>,
    pub classify: OperationConfig,
    pub generate: OperationConfig,
    pub providers: Vec<ProviderConfig>,
}

impl Config {
    pub fn provider(&self, name: &str) -> Option<&ProviderConfig> {
        self.providers.iter().find(|provider| provider.name == name)
    }

    pub fn classify_provider(&self) -> Result<&ProviderConfig, ConfigError> {
        let name = self.classify.provider.as_ref().or(self.default_provider.as_ref());
        let name = name.ok_or_else(|| ConfigError::new("No classify provider configured"))?;
        self.provider(name)
            .ok_or_else(|| ConfigError::new(format!("Unknown provider \"{name}\"")))
    }

    pub fn generate_provider(&self) -> Result<&ProviderConfig, ConfigError> {
        let name = self.generate.provider.as_ref().or(self.default_provider.as_ref());
        let name = name.ok_or_else(|| ConfigError::new("No generate provider configured"))?;
        self.provider(name)
            .ok_or_else(|| ConfigError::new(format!("Unknown provider \"{name}\"")))
    }
}

/// Resolve the current Git worktree root, falling back to `cwd` outside a repository.
pub fn worktree_root(cwd: &Path) -> PathBuf {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(cwd)
        .output();
    match output {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if text.is_empty() { cwd.to_path_buf() } else { PathBuf::from(text) }
        }
        _ => cwd.to_path_buf(),
    }
}

/// Load and merge global and project configuration.
pub fn load(cwd: &Path, explicit: Option<&Path>) -> Result<Config, ConfigError> {
    let _ = (cwd, explicit);
    todo!("lead: config")
}
