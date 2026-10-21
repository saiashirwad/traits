use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::{Map, Value, json};

use crate::config::{JsonMode, ProviderKind};
use crate::error::ToolFailure;

use super::{
    ClassifyAnswer, ClassifyQuestion, DecisionResponse, GenerateRequest, LanguageModel, Usage,
};

/// Probability drift tolerated per criteria label before a classification is
/// rejected, matching the TypeSafe provider.
const DRIFT_PER_LABEL: f64 = 0.005;
/// Below this drift the distribution is left untouched; above it the values are
/// rescaled to sum to one.
const RESCALE_THRESHOLD: f64 = 1e-6;

const CLASSIFY_SYSTEM: &str = concat!(
    "You are a decision model. For every question choose exactly one label from its criteria ",
    "and return only a JSON object of the form ",
    "{\"answers\":[{\"id\":\"<question id>\",\"label\":\"<chosen label>\",\"probabilities\":{\"<label>\":<number>}}]}. ",
    "Include one entry per question, use the exact question id, choose a label from that ",
    "question's criteria, and provide a probability for every label of that question. ",
    "Probabilities must be finite, between 0 and 1, and sum to 1. Return no other text."
);

/// OpenAI-compatible chat completions provider. Implements both classification
/// and structured generation.
pub struct OpenAiCompatProvider {
    pub client: reqwest::Client,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub kind: ProviderKind,
    pub json_mode: JsonMode,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub reasoning_effort: Option<String>,
    pub extra_headers: Vec<(String, String)>,
    /// Stable session identifier sent to OpenCode Go.
    pub session_id: String,
    /// Set once a `json_schema` request has been rejected, so later requests go
    /// straight to `json_object` mode.
    downgraded: AtomicBool,
}

impl OpenAiCompatProvider {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        client: reqwest::Client,
        base_url: String,
        api_key: String,
        model: String,
        kind: ProviderKind,
        json_mode: JsonMode,
        temperature: Option<f64>,
        max_tokens: Option<u32>,
        reasoning_effort: Option<String>,
        extra_headers: Vec<(String, String)>,
    ) -> Self {
        Self {
            client,
            base_url,
            api_key,
            model,
            kind,
            json_mode,
            temperature,
            max_tokens,
            reasoning_effort,
            extra_headers,
            session_id: uuid::Uuid::new_v4().to_string(),
            downgraded: AtomicBool::new(false),
        }
    }

    fn endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }

    fn request(&self, body: &Value) -> reqwest::RequestBuilder {
        let mut request = self
            .client
            .post(self.endpoint())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(body);
        if self.kind == ProviderKind::OpenCodeGo {
            request = request
                .header("x-opencode-session", self.session_id.clone())
                .header(
                    "user-agent",
                    format!("traits/{}", env!("CARGO_PKG_VERSION")),
                );
        }
        if self.kind == ProviderKind::OpenRouter {
            request = request
                .header("HTTP-Referer", "https://github.com/crosshatch/traits")
                .header("X-Title", "traits");
        }
        for (name, value) in &self.extra_headers {
            request = request.header(name, value);
        }
        request
    }

    async fn post(&self, body: &Value) -> Result<(reqwest::StatusCode, String), ToolFailure> {
        let response = self.request(body).send().await.map_err(|error| {
            ToolFailure::ai(format!("Chat completions request failed: {error}"))
        })?;
        let status = response.status();
        let text = response.text().await.map_err(|error| {
            ToolFailure::ai(format!("Failed to read chat completions response: {error}"))
        })?;
        Ok((status, text))
    }

    fn base_body(&self, messages: Value) -> Map<String, Value> {
        let mut body = Map::new();
        body.insert("model".to_string(), json!(self.model));
        body.insert("messages".to_string(), messages);
        if let Some(temperature) = self.temperature {
            body.insert("temperature".to_string(), json!(temperature));
        }
        if let Some(max_tokens) = self.max_tokens {
            body.insert("max_tokens".to_string(), json!(max_tokens));
        }
        if let Some(reasoning_effort) = &self.reasoning_effort {
            body.insert("reasoning_effort".to_string(), json!(reasoning_effort));
        }
        body
    }

    fn classify_body(&self, state: &Value, questions: &[(String, ClassifyQuestion)]) -> Value {
        let encoded_questions: Vec<Value> = questions
            .iter()
            .map(|(id, question)| {
                json!({
                    "id": id,
                    "instructions": question.instructions,
                    "criteria": question.criteria,
                })
            })
            .collect();
        let user = json!({ "state": state, "questions": encoded_questions });
        let messages = json!([
            { "role": "system", "content": CLASSIFY_SYSTEM },
            { "role": "user", "content": user.to_string() },
        ]);
        let mut body = self.base_body(messages);
        if !matches!(self.json_mode, JsonMode::Prompt) {
            body.insert(
                "response_format".to_string(),
                json!({ "type": "json_object" }),
            );
        }
        Value::Object(body)
    }

    fn generate_body(&self, request: &GenerateRequest, mode: JsonMode) -> Value {
        let (system, response_format) = match mode {
            JsonMode::Schema => (
                request.system.clone(),
                Some(json!({
                    "type": "json_schema",
                    "json_schema": {
                        "name": request.schema_name,
                        "schema": request.schema,
                        "strict": request.strict,
                    },
                })),
            ),
            JsonMode::Object => (
                append_schema(&request.system, &request.schema),
                Some(json!({ "type": "json_object" })),
            ),
            JsonMode::Prompt => (append_schema(&request.system, &request.schema), None),
        };
        let messages = json!([
            { "role": "system", "content": system },
            { "role": "user", "content": request.user },
        ]);
        let mut body = self.base_body(messages);
        if let Some(response_format) = response_format {
            body.insert("response_format".to_string(), response_format);
        }
        Value::Object(body)
    }
}

#[async_trait]
impl super::DecisionModel for OpenAiCompatProvider {
    async fn decide(
        &self,
        state: Value,
        questions: Vec<(String, ClassifyQuestion)>,
    ) -> Result<DecisionResponse, ToolFailure> {
        let body = self.classify_body(&state, &questions);
        let (status, text) = self.post(&body).await?;
        if !status.is_success() {
            return Err(ToolFailure::ai(format!(
                "Chat completions returned HTTP {status}: {text}"
            )));
        }
        let response: Value = serde_json::from_str(&text).map_err(|error| {
            ToolFailure::ai(format!(
                "Chat completions returned invalid JSON: {error}: {text}"
            ))
        })?;
        let content = message_content(&response)?;
        let payload: Value = serde_json::from_str(content).map_err(|error| {
            ToolFailure::ai(format!(
                "Provider returned invalid JSON: {error}: {content}"
            ))
        })?;

        let answers = validate_answers(&questions, &payload)?;
        let usage = Usage {
            input_tokens: response
                .pointer("/usage/prompt_tokens")
                .and_then(Value::as_u64),
            output_tokens: response
                .pointer("/usage/completion_tokens")
                .and_then(Value::as_u64),
        };
        Ok(DecisionResponse { answers, usage })
    }
}

#[async_trait]
impl LanguageModel for OpenAiCompatProvider {
    async fn generate_object(&self, request: GenerateRequest) -> Result<Value, ToolFailure> {
        let mode = if self.downgraded.load(Ordering::Relaxed) {
            JsonMode::Object
        } else {
            self.json_mode
        };
        let body = self.generate_body(&request, mode);
        let (status, text) = self.post(&body).await?;

        if mode == JsonMode::Schema && matches!(status.as_u16(), 400 | 422) {
            self.downgraded.store(true, Ordering::Relaxed);
            let retry = self.generate_body(&request, JsonMode::Object);
            let (status, text) = self.post(&retry).await?;
            return parse_generated(status, text);
        }
        parse_generated(status, text)
    }
}

fn parse_generated(status: reqwest::StatusCode, text: String) -> Result<Value, ToolFailure> {
    if !status.is_success() {
        return Err(ToolFailure::ai(format!(
            "Chat completions returned HTTP {status}: {text}"
        )));
    }
    let response: Value = serde_json::from_str(&text).map_err(|error| {
        ToolFailure::ai(format!(
            "Chat completions returned invalid JSON: {error}: {text}"
        ))
    })?;
    let content = message_content(&response)?;
    serde_json::from_str(content).map_err(|error| {
        ToolFailure::ai(format!(
            "Provider returned invalid JSON: {error}: {content}"
        ))
    })
}

fn message_content(response: &Value) -> Result<&str, ToolFailure> {
    response
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ToolFailure::ai(format!(
                "Chat completions returned no message content: {response}"
            ))
        })
}

fn append_schema(system: &str, schema: &Value) -> String {
    format!("{system}\n\nRespond with only a JSON object matching this JSON Schema:\n{schema}")
}

/// Validate the classification answers against the requested questions. Answer
/// ids must match exactly, labels must be known, and probabilities must be
/// finite, in range, and sum to one.
fn validate_answers(
    questions: &[(String, ClassifyQuestion)],
    payload: &Value,
) -> Result<IndexMap<String, ClassifyAnswer>, ToolFailure> {
    let raw_answers = payload
        .get("answers")
        .and_then(Value::as_array)
        .ok_or_else(|| ToolFailure::ai(format!("Provider returned no answers: {payload}")))?;

    let mut by_id: IndexMap<&str, &Value> = IndexMap::with_capacity(raw_answers.len());
    for answer in raw_answers {
        let id = answer.get("id").and_then(Value::as_str).ok_or_else(|| {
            ToolFailure::ai(format!(
                "Provider returned an answer without an id: {payload}"
            ))
        })?;
        if by_id.insert(id, answer).is_some() {
            return Err(ToolFailure::ai(format!(
                "Provider returned duplicate answers for decision \"{id}\""
            )));
        }
    }

    for (id, _) in questions {
        if !by_id.contains_key(id.as_str()) {
            return Err(ToolFailure::ai(format!(
                "Provider returned no answer for decision \"{id}\""
            )));
        }
    }
    if by_id.len() != questions.len() {
        let extra = by_id
            .keys()
            .find(|id| !questions.iter().any(|(requested, _)| requested == *id))
            .copied()
            .unwrap_or_default();
        return Err(ToolFailure::ai(format!(
            "Provider returned an answer for unknown decision \"{extra}\""
        )));
    }

    let mut answers = IndexMap::with_capacity(questions.len());
    for (id, question) in questions {
        let answer = by_id[id.as_str()];
        let label = answer
            .get("label")
            .and_then(Value::as_str)
            .filter(|label| question.criteria.contains_key(*label))
            .ok_or_else(|| {
                ToolFailure::ai(format!(
                    "Provider returned an unknown label for decision \"{id}\""
                ))
            })?;

        let raw_probabilities = answer.get("probabilities").and_then(Value::as_object);
        let mut probabilities = IndexMap::with_capacity(question.criteria.len());
        let mut total = 0.0;
        for criterion in question.criteria.keys() {
            let probability = raw_probabilities
                .and_then(|probabilities| probabilities.get(criterion))
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

        answers.insert(
            id.clone(),
            ClassifyAnswer {
                label: label.to_string(),
                probabilities,
                confidence: None,
            },
        );
    }
    Ok(answers)
}
