use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use kdl::{KdlDocument, KdlNode, KdlValue};
use serde::{Deserialize, Serialize};

use crate::error::ConfigError;
use crate::files::Excludes;

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

impl JsonMode {
    fn parse(value: &str) -> Result<Self, ConfigError> {
        match value {
            "schema" => Ok(Self::Schema),
            "object" => Ok(Self::Object),
            "prompt" => Ok(Self::Prompt),
            other => Err(ConfigError::new(format!(
                "Unknown json-mode \"{other}\"; expected schema, object, or prompt"
            ))),
        }
    }
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

impl ProviderKind {
    const ALL: [Self; 6] = [
        Self::Typesafe,
        Self::OpenCodeGo,
        Self::DeepSeek,
        Self::OpenRouter,
        Self::OpenAi,
        Self::Custom,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Typesafe => "typesafe",
            Self::OpenCodeGo => "opencode-go",
            Self::DeepSeek => "deepseek",
            Self::OpenRouter => "openrouter",
            Self::OpenAi => "openai",
            Self::Custom => "custom",
        }
    }

    fn parse(value: &str) -> Result<Self, ConfigError> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_str() == value)
            .ok_or_else(|| {
                ConfigError::new(format!(
                    "Unknown provider kind \"{value}\"; expected typesafe, opencode-go, deepseek, openrouter, openai, or custom"
                ))
            })
    }

    fn base_url(&self) -> Option<&'static str> {
        match self {
            Self::Typesafe => Some("https://api.typesafe.ai/v1"),
            Self::OpenCodeGo => Some("https://opencode.ai/inference/go/openai/v1"),
            Self::DeepSeek => Some("https://api.deepseek.com/v1"),
            Self::OpenRouter => Some("https://openrouter.ai/api/v1"),
            Self::OpenAi => Some("https://api.openai.com/v1"),
            Self::Custom => None,
        }
    }

    fn api_key_env(&self) -> &'static str {
        match self {
            Self::Typesafe => "TYPESAFE_API_KEY",
            Self::OpenCodeGo => "OPENCODE_API_KEY",
            Self::DeepSeek => "DEEPSEEK_API_KEY",
            Self::OpenRouter => "OPENROUTER_API_KEY",
            Self::OpenAi => "OPENAI_API_KEY",
            Self::Custom => "TRAITS_API_KEY",
        }
    }

    fn json_mode(&self) -> JsonMode {
        match self {
            Self::OpenRouter | Self::OpenAi => JsonMode::Schema,
            Self::Typesafe | Self::OpenCodeGo | Self::DeepSeek | Self::Custom => JsonMode::Object,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub name: String,
    pub kind: ProviderKind,
    pub model: String,
    pub api_key_env: String,
    pub base_url: String,
    pub json_mode: JsonMode,
    pub reasoning_effort: Option<String>,
    pub timeout_ms: u64,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct OperationConfig {
    /// The operation's provider, or the default provider when it names none.
    pub provider: Option<ProviderConfig>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub config_root: PathBuf,
    pub store: PathBuf,
    pub concurrency: NonZeroUsize,
    pub exclude_gitignored: bool,
    pub excludes: Excludes,
    /// Where `check_traits` caches answers; `None` asks every question.
    pub cache_dir: Option<PathBuf>,
    pub classify: OperationConfig,
    pub generate: OperationConfig,
    pub providers: Vec<ProviderConfig>,
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
            if text.is_empty() {
                cwd.to_path_buf()
            } else {
                PathBuf::from(text)
            }
        }
        _ => cwd.to_path_buf(),
    }
}

pub fn global_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("traits").join("config.kdl"))
}

/// Load and merge global and project configuration. Missing files are fine;
/// the resulting config may have no providers, which the provider builder
/// reports through the fallback tool handlers.
pub fn load(cwd: &Path, explicit: Option<&Path>) -> Result<Config, ConfigError> {
    let root = worktree_root(cwd);
    let project_path = explicit
        .map(Path::to_path_buf)
        .or_else(|| std::env::var_os("TRAITS_CONFIG").map(PathBuf::from))
        .or_else(|| {
            let candidate = root.join("traits.kdl");
            candidate.exists().then_some(candidate)
        });

    let global = match global_config_path() {
        Some(path) if path.exists() => parse_file(&path)?,
        _ => RawConfig::default(),
    };
    let project = match project_path {
        Some(path) if path.exists() => parse_file(&path)?,
        Some(path) if explicit.is_some() => {
            return Err(ConfigError::new(format!(
                "Config file {} does not exist",
                path.display()
            )));
        }
        _ => RawConfig::default(),
    };

    build(merge(global, project), &root)
}

pub fn parse_file(path: &Path) -> Result<RawConfig, ConfigError> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| ConfigError::new(format!("Unable to read {}: {error}", path.display())))?;
    parse_str(&text)
        .map_err(|error| ConfigError::new(format!("Invalid config {}: {error}", path.display())))
}

pub fn parse_str(text: &str) -> Result<RawConfig, ConfigError> {
    let document: KdlDocument = text
        .parse()
        .map_err(|error| ConfigError::new(format!("{error}")))?;
    let mut raw = RawConfig::default();
    for node in document.nodes() {
        match node.name().value() {
            "version" => raw.version = Some(int_arg(node, 0, "version")?),
            "store" => raw.store = Some(string_arg(node, 0, "store")?),
            "concurrency" => raw.concurrency = Some(int_arg(node, 0, "concurrency")?),
            "exclude-gitignored" => {
                raw.exclude_gitignored = Some(bool_flag(node, "exclude-gitignored")?)
            }
            "exclude" => raw.exclude.push(string_arg(node, 0, "exclude")?),
            "cache" => raw.cache = Some(bool_flag(node, "cache")?),
            "default-provider" => {
                raw.default_provider = Some(string_arg(node, 0, "default-provider")?)
            }
            "provider" => raw.provider.push(provider(node)?),
            "classify" => raw.classify = Some(operation(node, "classify")?),
            "generate" => raw.generate = Some(operation(node, "generate")?),
            other => {
                return Err(ConfigError::new(format!("Unknown config node \"{other}\"")));
            }
        }
    }
    Ok(raw)
}

fn provider(node: &KdlNode) -> Result<RawProvider, ConfigError> {
    let name = string_arg(node, 0, "provider name")?;
    let mut provider = RawProvider {
        name: name.clone(),
        kind: None,
        model: None,
        api_key_env: None,
        base_url: None,
        json_mode: None,
        reasoning_effort: None,
        timeout_ms: None,
        header: Vec::new(),
    };
    for child in children(node) {
        match child.name().value() {
            "kind" => provider.kind = Some(string_arg(child, 0, "kind")?),
            "model" => provider.model = Some(string_arg(child, 0, "model")?),
            "api-key-env" => provider.api_key_env = Some(string_arg(child, 0, "api-key-env")?),
            "base-url" => provider.base_url = Some(string_arg(child, 0, "base-url")?),
            "json-mode" => {
                provider.json_mode = Some(JsonMode::parse(&string_arg(child, 0, "json-mode")?)?)
            }
            "reasoning-effort" => {
                provider.reasoning_effort = Some(string_arg(child, 0, "reasoning-effort")?)
            }
            "timeout-ms" => provider.timeout_ms = Some(int_arg(child, 0, "timeout-ms")?),
            "header" => provider.header.push((
                string_arg(child, 0, "header name")?,
                string_arg(child, 1, "header value")?,
            )),
            other => {
                return Err(ConfigError::new(format!(
                    "Unknown node \"{other}\" in provider \"{name}\""
                )));
            }
        }
    }
    Ok(provider)
}

fn operation(node: &KdlNode, name: &str) -> Result<RawOperation, ConfigError> {
    let mut operation = RawOperation::default();
    for child in children(node) {
        match child.name().value() {
            "provider" => operation.provider = Some(string_arg(child, 0, "provider")?),
            "temperature" => operation.temperature = Some(float_arg(child, 0, "temperature")?),
            "max-tokens" => operation.max_tokens = Some(int_arg(child, 0, "max-tokens")?),
            other => {
                return Err(ConfigError::new(format!(
                    "Unknown node \"{other}\" in {name}"
                )));
            }
        }
    }
    Ok(operation)
}

fn children(node: &KdlNode) -> &[KdlNode] {
    node.children().map(KdlDocument::nodes).unwrap_or(&[])
}

fn string_arg(node: &KdlNode, index: usize, what: &str) -> Result<String, ConfigError> {
    match node.get(index) {
        Some(KdlValue::String(value)) => Ok(value.clone()),
        Some(other) => Err(ConfigError::new(format!(
            "{what} must be a string, received {other:?}"
        ))),
        None => Err(ConfigError::new(format!("{what} is required"))),
    }
}

fn int_arg<T: TryFrom<i128>>(node: &KdlNode, index: usize, what: &str) -> Result<T, ConfigError> {
    match node.get(index) {
        Some(KdlValue::Integer(value)) => T::try_from(*value)
            .map_err(|_| ConfigError::new(format!("{what} is out of range: {value}"))),
        Some(other) => Err(ConfigError::new(format!(
            "{what} must be an integer, received {other:?}"
        ))),
        None => Err(ConfigError::new(format!("{what} is required"))),
    }
}

fn float_arg(node: &KdlNode, index: usize, what: &str) -> Result<f64, ConfigError> {
    match node.get(index) {
        Some(KdlValue::Float(value)) => Ok(*value),
        Some(KdlValue::Integer(value)) => Ok(*value as f64),
        Some(other) => Err(ConfigError::new(format!(
            "{what} must be a number, received {other:?}"
        ))),
        None => Err(ConfigError::new(format!("{what} is required"))),
    }
}

fn bool_flag(node: &KdlNode, what: &str) -> Result<bool, ConfigError> {
    match (node.get(0), node.children()) {
        (None, None) => Ok(true),
        (Some(KdlValue::Bool(value)), None) => Ok(*value),
        _ => Err(ConfigError::new(format!(
            "{what} takes no argument, or #true/#false"
        ))),
    }
}

pub fn merge(global: RawConfig, project: RawConfig) -> RawConfig {
    let mut providers = global.provider;
    for incoming in project.provider {
        match providers
            .iter_mut()
            .find(|provider| provider.name == incoming.name)
        {
            Some(existing) => *existing = incoming,
            None => providers.push(incoming),
        }
    }
    let mut excludes = global.exclude;
    excludes.extend(project.exclude);
    RawConfig {
        version: project.version.or(global.version),
        store: project.store.or(global.store),
        concurrency: project.concurrency.or(global.concurrency),
        exclude_gitignored: project.exclude_gitignored.or(global.exclude_gitignored),
        exclude: excludes,
        cache: project.cache.or(global.cache),
        default_provider: project.default_provider.or(global.default_provider),
        provider: providers,
        classify: merge_operation(global.classify, project.classify),
        generate: merge_operation(global.generate, project.generate),
    }
}

fn merge_operation(
    global: Option<RawOperation>,
    project: Option<RawOperation>,
) -> Option<RawOperation> {
    match (global, project) {
        (None, None) => None,
        (Some(global), None) => Some(global),
        (None, Some(project)) => Some(project),
        (Some(global), Some(project)) => Some(RawOperation {
            provider: project.provider.or(global.provider),
            temperature: project.temperature.or(global.temperature),
            max_tokens: project.max_tokens.or(global.max_tokens),
        }),
    }
}

pub fn build(raw: RawConfig, root: &Path) -> Result<Config, ConfigError> {
    let mut providers = Vec::new();
    for raw_provider in raw.provider {
        let kind_name = raw_provider.kind.ok_or_else(|| {
            ConfigError::new(format!("Provider \"{}\" requires kind", raw_provider.name))
        })?;
        let kind = ProviderKind::parse(&kind_name)?;
        let model = raw_provider.model.ok_or_else(|| {
            ConfigError::new(format!("Provider \"{}\" requires model", raw_provider.name))
        })?;
        let base_url = raw_provider
            .base_url
            .or_else(|| kind.base_url().map(str::to_string))
            .ok_or_else(|| {
                ConfigError::new(format!(
                    "Provider \"{}\" requires base-url",
                    raw_provider.name
                ))
            })?;
        providers.push(ProviderConfig {
            name: raw_provider.name,
            kind,
            model,
            api_key_env: raw_provider
                .api_key_env
                .unwrap_or_else(|| kind.api_key_env().to_string()),
            base_url,
            json_mode: raw_provider.json_mode.unwrap_or_else(|| kind.json_mode()),
            reasoning_effort: raw_provider.reasoning_effort,
            timeout_ms: raw_provider.timeout_ms.unwrap_or(120_000),
            headers: raw_provider.header,
        });
    }

    let store = match raw.store {
        Some(store) => {
            let path = PathBuf::from(store);
            if path.is_absolute() {
                path
            } else {
                root.join(path)
            }
        }
        None => root.join(".agents/traits"),
    };

    let operation = |raw_operation: Option<RawOperation>| {
        let raw_operation = raw_operation.unwrap_or_default();
        let provider = raw_operation
            .provider
            .or_else(|| raw.default_provider.clone())
            .map(|name| {
                providers
                    .iter()
                    .find(|provider| provider.name == name)
                    .cloned()
                    .ok_or_else(|| ConfigError::new(format!("Unknown provider \"{name}\"")))
            })
            .transpose()?;
        Ok::<_, ConfigError>(OperationConfig {
            provider,
            temperature: raw_operation.temperature,
            max_tokens: raw_operation.max_tokens,
        })
    };
    let classify = operation(raw.classify)?;
    let generate = operation(raw.generate)?;

    Ok(Config {
        config_root: root.to_path_buf(),
        store,
        concurrency: NonZeroUsize::new(usize::from(raw.concurrency.unwrap_or(4)))
            .unwrap_or(NonZeroUsize::MIN),
        exclude_gitignored: raw.exclude_gitignored.unwrap_or(true),
        excludes: Excludes::compile(root, raw.exclude)?,
        cache_dir: if raw.cache.unwrap_or(true) {
            dirs::cache_dir().map(|dir| dir.join("traits"))
        } else {
            None
        },
        classify,
        generate,
        providers,
    })
}

#[derive(Debug, Clone, Default)]
pub struct RawConfig {
    pub version: Option<u32>,
    pub store: Option<String>,
    pub concurrency: Option<u16>,
    pub exclude_gitignored: Option<bool>,
    pub exclude: Vec<String>,
    pub cache: Option<bool>,
    pub default_provider: Option<String>,
    pub provider: Vec<RawProvider>,
    pub classify: Option<RawOperation>,
    pub generate: Option<RawOperation>,
}

#[derive(Debug, Clone)]
pub struct RawProvider {
    pub name: String,
    pub kind: Option<String>,
    pub model: Option<String>,
    pub api_key_env: Option<String>,
    pub base_url: Option<String>,
    pub json_mode: Option<JsonMode>,
    pub reasoning_effort: Option<String>,
    pub timeout_ms: Option<u64>,
    pub header: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct RawOperation {
    pub provider: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        PathBuf::from("/repo")
    }

    fn provider<'a>(config: &'a Config, name: &str) -> &'a ProviderConfig {
        config
            .providers
            .iter()
            .find(|provider| provider.name == name)
            .unwrap()
    }

    #[test]
    fn parses_providers_operations_and_defaults() {
        let raw = parse_str(
            r#"
            version 1
            concurrency 8
            exclude "repos/**"
            default-provider "opencode"

            provider "opencode" {
                kind "opencode-go"
                model "deepseek-v4.1-flash"
            }

            provider "decisions" {
                kind "typesafe"
                model "jev-latest"
                api-key-env "TYPESAFE_API_KEY"
                timeout-ms 30000
                header "x-extra" "value"
            }

            classify {
                provider "decisions"
                temperature 0
                max-tokens 1024
            }
            "#,
        )
        .unwrap();
        let config = build(raw, &root()).unwrap();
        assert_eq!(config.concurrency.get(), 8);
        assert_eq!(config.providers.len(), 2);
        let opencode = provider(&config, "opencode");
        assert_eq!(opencode.kind, ProviderKind::OpenCodeGo);
        assert_eq!(
            opencode.base_url,
            "https://opencode.ai/inference/go/openai/v1"
        );
        assert_eq!(opencode.api_key_env, "OPENCODE_API_KEY");
        assert_eq!(opencode.json_mode, JsonMode::Object);
        let decisions = provider(&config, "decisions");
        assert_eq!(decisions.timeout_ms, 30_000);
        assert_eq!(
            decisions.headers,
            vec![("x-extra".to_string(), "value".to_string())]
        );
        assert_eq!(config.classify.provider.as_ref().unwrap().name, "decisions");
        assert_eq!(
            config.generate.provider.as_ref().unwrap().name,
            "opencode",
            "generate falls back to the default provider"
        );
        assert_eq!(config.classify.max_tokens, Some(1024));
        assert_eq!(config.excludes.patterns(), ["repos/**"]);
        assert_eq!(config.store, root().join(".agents/traits"));
        assert!(config.exclude_gitignored);
    }

    #[test]
    fn single_provider_and_gitignored_flags() {
        let raw = parse_str(
            r#"
            exclude-gitignored
            provider "only" {
                kind "deepseek"
                model "deepseek-flash"
            }
            "#,
        )
        .unwrap();
        let config = build(raw, &root()).unwrap();
        assert_eq!(config.providers.len(), 1);
        assert!(config.exclude_gitignored);

        let raw = parse_str("exclude-gitignored #false").unwrap();
        let config = build(raw, &root()).unwrap();
        assert!(!config.exclude_gitignored);
    }

    #[test]
    fn caches_under_the_user_cache_dir_unless_disabled() {
        let config = build(RawConfig::default(), &root()).unwrap();
        assert_eq!(
            config.cache_dir,
            dirs::cache_dir().map(|dir| dir.join("traits"))
        );

        let disabled = merge(
            parse_str("cache").unwrap(),
            parse_str("cache #false").unwrap(),
        );
        assert_eq!(build(disabled, &root()).unwrap().cache_dir, None);
    }

    #[test]
    fn project_overrides_global_and_merges_providers() {
        let global = parse_str(
            r#"
            concurrency 2
            provider "a" {
                kind "openai"
                model "gpt-a"
            }
            provider "b" {
                kind "deepseek"
                model "deepseek-flash"
            }
            exclude "global/**"
            "#,
        )
        .unwrap();
        let project = parse_str(
            r#"
            concurrency 8
            provider "a" {
                kind "openai"
                model "gpt-b"
            }
            exclude "project/**"
            "#,
        )
        .unwrap();
        let config = build(merge(global, project), &root()).unwrap();
        assert_eq!(config.concurrency.get(), 8);
        assert_eq!(config.providers.len(), 2);
        assert_eq!(provider(&config, "a").model, "gpt-b");
        assert_eq!(config.excludes.patterns(), ["global/**", "project/**"]);
    }

    #[test]
    fn rejects_unknown_nodes_and_bad_kinds() {
        assert!(parse_str("concurreny 4").is_err());
        let raw = parse_str(r#"provider "x" { kind "mystery" model "m" }"#).unwrap();
        assert!(build(raw, &root()).is_err());
    }

    #[test]
    fn rejects_unknown_provider_names_and_out_of_range_integers() {
        let raw = parse_str(
            r#"provider "x" { kind "openai"; model "m" }
            classify { provider "nope" }"#,
        )
        .unwrap();
        let error = build(raw, &root()).unwrap_err();
        assert_eq!(error.to_string(), "Unknown provider \"nope\"");

        let raw = parse_str(
            r#"provider "x" { kind "openai"; model "m" }
            default-provider "missing""#,
        )
        .unwrap();
        assert!(build(raw, &root()).is_err());

        for text in [
            "concurrency -1",
            "concurrency 70000",
            "classify { max-tokens -5 }",
        ] {
            let error = parse_str(text).unwrap_err();
            assert!(
                error.to_string().contains("out of range"),
                "{text}: {error}"
            );
        }
        let config = build(parse_str("concurrency 0").unwrap(), &root()).unwrap();
        assert_eq!(config.concurrency.get(), 1);
    }

    #[test]
    fn rejects_invalid_exclude_patterns_at_load() {
        let raw = parse_str(r#"exclude "src/[unclosed""#).unwrap();
        let error = build(raw, &root()).unwrap_err();
        assert!(
            error.to_string().contains("Invalid exclude pattern"),
            "{error}"
        );
    }

    #[test]
    fn custom_provider_requires_base_url() {
        let raw = parse_str(r#"provider "x" { kind "custom" model "m" }"#).unwrap();
        assert!(build(raw, &root()).is_err());
    }
}
