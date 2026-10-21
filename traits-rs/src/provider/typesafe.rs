use async_trait::async_trait;
use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::ToolFailure;

use super::{DecisionResponse, Question, Usage, criteria_json, validate_answer};

#[derive(Debug, Deserialize)]
struct SystemOneResponse {
    #[serde(default)]
    answers: IndexMap<String, SystemOneAnswer>,
    #[serde(default)]
    usage: Option<SystemOneUsage>,
}

#[derive(Debug, Deserialize)]
struct SystemOneAnswer {
    #[serde(default)]
    choice: Value,
    #[serde(default)]
    probabilities: IndexMap<String, Value>,
    #[serde(default)]
    confidence: Value,
}

#[derive(Debug, Deserialize)]
struct SystemOneUsage {
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
}

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
        state: &Value,
        questions: &[Question<'_>],
    ) -> Result<DecisionResponse, ToolFailure> {
        let encoded_questions = questions
            .iter()
            .map(|question| {
                (
                    question.id.to_string(),
                    json!({
                        "type": "choice",
                        "instructions": question.instructions,
                        "criteria": criteria_json(question.criteria),
                    }),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        let body = json!({
            "model": self.model,
            "state": state,
            "questions": encoded_questions,
        });

        let url = format!("{}/systemone", self.base_url.trim_end_matches('/'));
        let mut request = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept", "application/json")
            .json(&body);
        for (name, value) in &self.extra_headers {
            request = request.header(name, value);
        }

        let response = request
            .send()
            .await
            .map_err(|error| ToolFailure::ai(format!("TypeSafe request failed: {error}")))?;
        let status = response.status();
        let text = response.text().await.map_err(|error| {
            ToolFailure::ai(format!("Failed to read TypeSafe response: {error}"))
        })?;
        if !status.is_success() {
            return Err(ToolFailure::ai(format!(
                "TypeSafe returned HTTP {status}: {text}"
            )));
        }

        let parsed: SystemOneResponse = serde_json::from_str(&text).map_err(|error| {
            ToolFailure::ai(format!("TypeSafe returned invalid JSON: {error}: {text}"))
        })?;

        let answers = questions
            .iter()
            .map(|question| {
                let id = question.id;
                let answer = parsed.answers.get(id).ok_or_else(|| {
                    ToolFailure::ai(format!("Provider returned no answer for decision \"{id}\""))
                })?;
                let confidence = match &answer.confidence {
                    Value::Null => None,
                    value => Some(
                        value
                            .as_f64()
                            .filter(|confidence| {
                                confidence.is_finite() && (0.0..=1.0).contains(confidence)
                            })
                            .ok_or_else(|| {
                                ToolFailure::ai(format!(
                                    "Provider returned confidence outside [0, 1] for decision \"{id}\""
                                ))
                            })?,
                    ),
                };
                validate_answer(
                    question,
                    answer.choice.as_str(),
                    |label| answer.probabilities.get(label).and_then(Value::as_f64),
                    confidence,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let usage = parsed
            .usage
            .map(|usage| Usage {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
            })
            .unwrap_or_default();

        Ok(DecisionResponse { answers, usage })
    }
}
