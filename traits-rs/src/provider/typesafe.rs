use async_trait::async_trait;
use serde_json::Value;

use crate::error::ToolFailure;

use super::{ClassifyQuestion, DecisionResponse};

/// TypeSafe System One decision provider (choice questions).
pub struct TypeSafeProvider {
    pub client: reqwest::Client,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub extra_headers: Vec<(String, String)>,
}

#[async_trait]
impl super::DecisionModel for TypeSafeProvider {
    async fn decide(
        &self,
        state: Value,
        questions: Vec<(String, ClassifyQuestion)>,
    ) -> Result<DecisionResponse, ToolFailure> {
        let _ = (state, questions);
        todo!("delegate: provider")
    }
}
