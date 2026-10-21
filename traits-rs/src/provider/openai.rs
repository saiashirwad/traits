use async_trait::async_trait;
use serde_json::Value;

use crate::error::ToolFailure;

use super::{ClassifyQuestion, DecisionResponse, GenerateRequest, LanguageModel};

/// OpenAI-compatible chat completions provider. Implements both classification
/// and structured generation.
pub struct OpenAiCompatProvider {
    pub client: reqwest::Client,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub json_mode: crate::config::JsonMode,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub reasoning_effort: Option<String>,
    pub extra_headers: Vec<(String, String)>,
}

#[async_trait]
impl super::DecisionModel for OpenAiCompatProvider {
    async fn decide(
        &self,
        state: Value,
        questions: Vec<(String, ClassifyQuestion)>,
    ) -> Result<DecisionResponse, ToolFailure> {
        let _ = (state, questions);
        todo!("delegate: provider")
    }
}

#[async_trait]
impl LanguageModel for OpenAiCompatProvider {
    async fn generate_object(&self, request: GenerateRequest) -> Result<Value, ToolFailure> {
        let _ = request;
        todo!("delegate: provider")
    }
}
