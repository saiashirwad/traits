use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::{Map, Value, json};

use crate::config::{JsonMode, ProviderKind};
use crate::error::ToolFailure;

use super::{
    Answer, DecisionResponse, GenerateRequest, LanguageModel, Question, Usage, criteria_json,
    validate_answer,
};

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

    fn classify_body(&self, state: &Value, questions: &[Question<'_>]) -> Value {
        let encoded_questions: Vec<Value> = questions
            .iter()
            .map(|question| {
                json!({
                    "id": question.id,
                    "instructions": question.instructions,
                    "criteria": criteria_json(question.criteria),
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
        state: &Value,
        questions: &[Question<'_>],
    ) -> Result<DecisionResponse, ToolFailure> {
        let body = self.classify_body(state, questions);
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

        let answers = validate_answers(questions, &payload)?;
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

/// Match the classification answers to the requested questions: ids must
/// match exactly, with no duplicates or extras, before each answer is validated.
fn validate_answers(
    questions: &[Question<'_>],
    payload: &Value,
) -> Result<Vec<Answer>, ToolFailure> {
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

    let answers = questions
        .iter()
        .map(|question| {
            let id = question.id;
            let answer = by_id.get(id).ok_or_else(|| {
                ToolFailure::ai(format!("Provider returned no answer for decision \"{id}\""))
            })?;
            let probabilities = answer.get("probabilities").and_then(Value::as_object);
            validate_answer(
                question,
                answer.get("label").and_then(Value::as_str),
                |label| {
                    probabilities
                        .and_then(|map| map.get(label))
                        .and_then(Value::as_f64)
                },
                None,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    if let Some(extra) = by_id
        .keys()
        .find(|id| !questions.iter().any(|question| question.id == **id))
    {
        return Err(ToolFailure::ai(format!(
            "Provider returned an answer for unknown decision \"{extra}\""
        )));
    }
    Ok(answers)
}
