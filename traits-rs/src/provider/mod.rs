use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::Value;

use crate::config::{Config, ProviderConfig, ProviderKind};
use crate::error::{ConfigError, ToolFailure};

use openai::OpenAiCompatProvider;
use typesafe::TypeSafeProvider;

pub mod openai;
pub mod typesafe;

#[derive(Debug, Clone, PartialEq)]
pub struct ClassifyQuestion {
    pub instructions: String,
    pub criteria: IndexMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassifyAnswer {
    pub label: String,
    pub probabilities: IndexMap<String, f64>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Usage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct DecisionResponse {
    pub answers: IndexMap<String, ClassifyAnswer>,
    pub usage: Usage,
}

#[async_trait]
pub trait DecisionModel: Send + Sync {
    async fn decide(
        &self,
        state: Value,
        questions: Vec<(String, ClassifyQuestion)>,
    ) -> Result<DecisionResponse, ToolFailure>;
}

#[derive(Debug, Clone)]
pub struct GenerateRequest {
    pub system: String,
    pub user: String,
    pub schema_name: String,
    pub schema: Value,
    pub strict: bool,
}

#[async_trait]
pub trait LanguageModel: Send + Sync {
    async fn generate_object(&self, request: GenerateRequest) -> Result<Value, ToolFailure>;
}

pub struct Models {
    pub decision: Arc<dyn DecisionModel>,
    /// Absent when no generate provider is configured. Tools that only
    /// classify (check_traits) still work.
    pub language: Option<Arc<dyn LanguageModel>>,
}

/// Build the decision and language models selected by configuration.
pub fn build(config: &Config) -> Result<Models, ConfigError> {
    let classify = config.classify_provider()?;

    let decision: Arc<dyn DecisionModel> = match classify.kind {
        ProviderKind::Typesafe => Arc::new(TypeSafeProvider {
            client: build_client(classify.timeout_ms)?,
            base_url: resolve_base_url(classify)?,
            api_key: resolve_api_key(classify)?,
            model: classify.model.clone(),
            extra_headers: classify.headers.clone(),
        }),
        _ => Arc::new(OpenAiCompatProvider::new(
            build_client(classify.timeout_ms)?,
            resolve_base_url(classify)?,
            resolve_api_key(classify)?,
            classify.model.clone(),
            classify.kind,
            classify.json_mode,
            Some(config.classify.temperature.unwrap_or(0.0)),
            Some(config.classify.max_tokens.unwrap_or(1024)),
            classify.reasoning_effort.clone(),
            classify.headers.clone(),
        )),
    };

    let language: Option<Arc<dyn LanguageModel>> = match config
        .generate
        .provider
        .as_ref()
        .or(config.default_provider.as_ref())
    {
        None => None,
        Some(name) => {
            let generate = config
                .provider(name)
                .ok_or_else(|| ConfigError::new(format!("Unknown provider \"{name}\"")))?;
            match generate.kind {
                // TypeSafe is decision-only; treat it as "no language model"
                // so classify-only configurations still serve check_traits.
                ProviderKind::Typesafe => None,
                _ => Some(Arc::new(OpenAiCompatProvider::new(
                    build_client(generate.timeout_ms)?,
                    resolve_base_url(generate)?,
                    resolve_api_key(generate)?,
                    generate.model.clone(),
                    generate.kind,
                    generate.json_mode,
                    Some(config.generate.temperature.unwrap_or(0.2)),
                    Some(config.generate.max_tokens.unwrap_or(8192)),
                    generate.reasoning_effort.clone(),
                    generate.headers.clone(),
                ))),
            }
        }
    };

    Ok(Models { decision, language })
}

fn build_client(timeout_ms: u64) -> Result<reqwest::Client, ConfigError> {
    let mut builder = reqwest::Client::builder();
    if timeout_ms > 0 {
        builder = builder.timeout(Duration::from_millis(timeout_ms));
    }
    builder
        .build()
        .map_err(|error| ConfigError::new(format!("Failed to build HTTP client: {error}")))
}

fn resolve_api_key(provider: &ProviderConfig) -> Result<String, ConfigError> {
    match std::env::var(&provider.api_key_env) {
        Ok(value) if !value.is_empty() => Ok(value),
        _ => Err(ConfigError::new(format!(
            "Missing API key for provider \"{}\": environment variable {} is not set",
            provider.name, provider.api_key_env
        ))),
    }
}

fn resolve_base_url(provider: &ProviderConfig) -> Result<String, ConfigError> {
    if !provider.base_url.is_empty() {
        return Ok(provider.base_url.clone());
    }
    default_base_url(provider.kind)
        .map(str::to_string)
        .ok_or_else(|| {
            ConfigError::new(format!(
                "Provider \"{}\" requires a base URL",
                provider.name
            ))
        })
}

fn default_base_url(kind: ProviderKind) -> Option<&'static str> {
    match kind {
        ProviderKind::Typesafe => Some("https://api.typesafe.ai/v1"),
        ProviderKind::OpenCodeGo => Some("https://opencode.ai/inference/go/openai/v1"),
        ProviderKind::DeepSeek => Some("https://api.deepseek.com/v1"),
        ProviderKind::OpenRouter => Some("https://openrouter.ai/api/v1"),
        ProviderKind::OpenAi => Some("https://api.openai.com/v1"),
        ProviderKind::Custom => None,
    }
}
