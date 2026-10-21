use std::sync::Arc;

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::Value;

use crate::config::Config;
use crate::error::{ConfigError, ToolFailure};

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
    pub language: Arc<dyn LanguageModel>,
}

/// Build the decision and language models selected by configuration.
pub fn build(config: &Config) -> Result<Models, ConfigError> {
    let _ = config;
    todo!("delegate: provider")
}
