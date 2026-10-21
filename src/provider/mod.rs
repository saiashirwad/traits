use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Map, Value};

use crate::config::{Config, ProviderConfig, ProviderKind};
use crate::error::{ConfigError, ToolFailure};

use openai::OpenAiCompatProvider;
use typesafe::TypeSafeProvider;

pub mod openai;
pub mod typesafe;

/// Probability drift tolerated per criteria label before a distribution is
/// rejected, matching Effect's `probabilityPrecision` of 2.
const DRIFT_PER_LABEL: f64 = 0.005;
/// Below this drift the distribution is left untouched; above it the values are
/// rescaled to sum to one.
const RESCALE_THRESHOLD: f64 = 1e-6;

/// One labeled outcome a decision question may choose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Criterion {
    pub label: &'static str,
    pub description: &'static str,
}

/// A closed set of decision outcomes. `ALL[i]` is the outcome `CRITERIA[i]`
/// describes.
pub trait Choice: Copy + 'static {
    const ALL: &'static [Self];
    const CRITERIA: &'static [Criterion];
    const ALIGNED: () = assert!(Self::ALL.len() == Self::CRITERIA.len());
}

#[derive(Debug, Clone, Copy)]
pub struct Question<'a> {
    pub id: &'a str,
    pub instructions: &'a str,
    pub criteria: &'static [Criterion],
}

/// A validated answer. `choice` and `probabilities` index the question's criteria.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub choice: usize,
    pub probabilities: Vec<f64>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Usage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct DecisionResponse {
    /// One answer per question, in question order.
    pub answers: Vec<Answer>,
    pub usage: Usage,
}

#[async_trait]
pub trait DecisionModel: Send + Sync {
    async fn decide(
        &self,
        state: &Value,
        questions: &[Question<'_>],
    ) -> Result<DecisionResponse, ToolFailure>;
}

/// Ask one question and decode the chosen outcome.
pub async fn ask<C: Choice>(
    model: &dyn DecisionModel,
    state: &Value,
    id: &str,
    instructions: &str,
) -> Result<C, ToolFailure> {
    let () = C::ALIGNED;
    let question = Question {
        id,
        instructions,
        criteria: C::CRITERIA,
    };
    let answers = decide(model, state, &[question]).await?;
    Ok(C::ALL[answers[0].choice])
}

/// Ask `questions` and return one answer per question, in order, each sized to
/// its question's criteria, so callers can index answers without checking.
pub async fn decide(
    model: &dyn DecisionModel,
    state: &Value,
    questions: &[Question<'_>],
) -> Result<Vec<Answer>, ToolFailure> {
    let answers = model.decide(state, questions).await?.answers;
    if answers.len() != questions.len() {
        return Err(ToolFailure::ai(format!(
            "Decision model returned {} answers for {} questions",
            answers.len(),
            questions.len()
        )));
    }
    for (question, answer) in questions.iter().zip(&answers) {
        let size = question.criteria.len();
        if answer.choice >= size || answer.probabilities.len() != size {
            return Err(ToolFailure::ai(format!(
                "Decision model returned a malformed answer for \"{}\"",
                question.id
            )));
        }
    }
    Ok(answers)
}

fn criteria_json(criteria: &[Criterion]) -> Value {
    criteria
        .iter()
        .map(|criterion| {
            (
                criterion.label.to_string(),
                Value::String(criterion.description.to_string()),
            )
        })
        .collect::<Map<_, _>>()
        .into()
}

/// Validate a provider's raw answer: the label must be one of the question's
/// criteria, and every criterion needs a probability in [0, 1] with the total
/// within drift of one, rescaled when it drifts.
fn validate_answer(
    question: &Question<'_>,
    label: Option<&str>,
    probability: impl Fn(&str) -> Option<f64>,
    confidence: Option<f64>,
) -> Result<Answer, ToolFailure> {
    let id = question.id;
    let choice = label
        .and_then(|label| {
            question
                .criteria
                .iter()
                .position(|criterion| criterion.label == label)
        })
        .ok_or_else(|| {
            ToolFailure::ai(format!(
                "Provider returned an unknown label for decision \"{id}\""
            ))
        })?;

    let mut probabilities = question
        .criteria
        .iter()
        .map(|criterion| {
            probability(criterion.label)
                .filter(|probability| probability.is_finite() && (0.0..=1.0).contains(probability))
                .ok_or_else(|| {
                    ToolFailure::ai(format!(
                        "Provider returned no probability for label \"{}\" of decision \"{id}\"",
                        criterion.label
                    ))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let total: f64 = probabilities.iter().sum();
    let tolerance = question.criteria.len() as f64 * DRIFT_PER_LABEL + RESCALE_THRESHOLD;
    let drift = (total - 1.0).abs();
    if total == 0.0 || drift > tolerance {
        return Err(ToolFailure::ai(format!(
            "Provider returned probabilities that do not sum to 1 for decision \"{id}\""
        )));
    }
    if drift > RESCALE_THRESHOLD {
        for probability in &mut probabilities {
            *probability /= total;
        }
    }

    Ok(Answer {
        choice,
        probabilities,
        confidence,
    })
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
    let classify = config
        .classify
        .provider
        .as_ref()
        .ok_or_else(|| ConfigError::new("No classify provider configured"))?;

    let decision: Arc<dyn DecisionModel> = match classify.kind {
        ProviderKind::Typesafe => Arc::new(TypeSafeProvider {
            client: build_client(classify.timeout_ms)?,
            base_url: classify.base_url.clone(),
            api_key: resolve_api_key(classify)?,
            model: classify.model.clone(),
            extra_headers: classify.headers.clone(),
        }),
        _ => Arc::new(OpenAiCompatProvider::new(
            build_client(classify.timeout_ms)?,
            classify.base_url.clone(),
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

    let language: Option<Arc<dyn LanguageModel>> = match &config.generate.provider {
        None => None,
        Some(generate) => {
            match generate.kind {
                // TypeSafe is decision-only; treat it as "no language model"
                // so classify-only configurations still serve check_traits.
                ProviderKind::Typesafe => None,
                _ => Some(Arc::new(OpenAiCompatProvider::new(
                    build_client(generate.timeout_ms)?,
                    generate.base_url.clone(),
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
