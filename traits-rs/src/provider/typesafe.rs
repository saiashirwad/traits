use async_trait::async_trait;
use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::ToolFailure;

use super::{ClassifyAnswer, ClassifyQuestion, DecisionResponse, Usage};

/// Probability drift tolerated per criteria label before a distribution is
/// rejected, matching Effect's `probabilityPrecision` of 2.
const DRIFT_PER_LABEL: f64 = 0.005;
/// Below this drift the distribution is left untouched; above it the values are
/// rescaled to sum to one.
const RESCALE_THRESHOLD: f64 = 1e-6;

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
        state: Value,
        questions: Vec<(String, ClassifyQuestion)>,
    ) -> Result<DecisionResponse, ToolFailure> {
        let mut encoded_questions = serde_json::Map::with_capacity(questions.len());
        for (id, question) in &questions {
            encoded_questions.insert(
                id.clone(),
                json!({
                    "type": "choice",
                    "instructions": question.instructions,
                    "criteria": question.criteria,
                }),
            );
        }
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

        let answers = validate_answers(&questions, &parsed.answers)?;
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

/// Validate the System One answers against the requested questions, mirroring
/// the Effect `DecisionModel` checks.
fn validate_answers(
    questions: &[(String, ClassifyQuestion)],
    raw_answers: &IndexMap<String, SystemOneAnswer>,
) -> Result<IndexMap<String, ClassifyAnswer>, ToolFailure> {
    let mut answers = IndexMap::with_capacity(questions.len());
    for (id, question) in questions {
        let answer = raw_answers.get(id).ok_or_else(|| {
            ToolFailure::ai(format!("Provider returned no answer for decision \"{id}\""))
        })?;

        let label = answer
            .choice
            .as_str()
            .filter(|label| question.criteria.contains_key(*label))
            .ok_or_else(|| {
                ToolFailure::ai(format!(
                    "Provider returned an unknown label for decision \"{id}\""
                ))
            })?;

        let mut probabilities = IndexMap::with_capacity(question.criteria.len());
        let mut total = 0.0;
        for criterion in question.criteria.keys() {
            let probability = answer
                .probabilities
                .get(criterion)
                .and_then(Value::as_f64)
                .filter(|probability| probability.is_finite() && (0.0..=1.0).contains(probability))
                .ok_or_else(|| {
                    ToolFailure::ai(format!(
                        "Provider returned no probability for label \"{criterion}\" of decision \"{id}\""
                    ))
                })?;
            total += probability;
            probabilities.insert(criterion.clone(), probability);
        }

        let tolerance = question.criteria.len() as f64 * DRIFT_PER_LABEL + RESCALE_THRESHOLD;
        let drift = (total - 1.0).abs();
        if total == 0.0 || drift > tolerance {
            return Err(ToolFailure::ai(format!(
                "Provider returned probabilities that do not sum to 1 for decision \"{id}\""
            )));
        }
        if drift > RESCALE_THRESHOLD {
            for probability in probabilities.values_mut() {
                *probability /= total;
            }
        }

        let confidence = match &answer.confidence {
            Value::Null => None,
            value => Some(
                value
                    .as_f64()
                    .filter(|confidence| confidence.is_finite() && (0.0..=1.0).contains(confidence))
                    .ok_or_else(|| {
                        ToolFailure::ai(format!(
                            "Provider returned confidence outside [0, 1] for decision \"{id}\""
                        ))
                    })?,
            ),
        };

        answers.insert(
            id.clone(),
            ClassifyAnswer {
                label: label.to_string(),
                probabilities,
                confidence,
            },
        );
    }
    Ok(answers)
}
