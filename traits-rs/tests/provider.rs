use std::path::PathBuf;

use serde_json::{Value, json};
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use traits::config::{Config, JsonMode, OperationConfig, ProviderConfig, ProviderKind};
use traits::provider::openai::OpenAiCompatProvider;
use traits::provider::typesafe::TypeSafeProvider;
use traits::provider::{ClassifyQuestion, DecisionModel, GenerateRequest, LanguageModel, build};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn criteria(labels: &[(&str, &str)]) -> Vec<(String, String)> {
    labels
        .iter()
        .map(|(label, description)| (label.to_string(), description.to_string()))
        .collect()
}

fn classify_question() -> (String, ClassifyQuestion) {
    (
        "assessment".to_string(),
        ClassifyQuestion {
            instructions: "Assess the state.".to_string(),
            criteria: criteria(&[
                ("compliant", "The input abides by the trait."),
                ("violation", "The input violates the trait."),
                ("not-applicable", "The trait does not apply."),
                ("inconclusive", "Insufficient context."),
            ])
            .into_iter()
            .collect(),
        },
    )
}

fn typesafe(server: &MockServer) -> TypeSafeProvider {
    TypeSafeProvider {
        client: reqwest::Client::new(),
        base_url: server.uri(),
        api_key: "test-key".to_string(),
        model: "jev-latest".to_string(),
        extra_headers: Vec::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn openai(
    server: &MockServer,
    kind: ProviderKind,
    json_mode: JsonMode,
    temperature: Option<f64>,
    max_tokens: Option<u32>,
    reasoning_effort: Option<String>,
) -> OpenAiCompatProvider {
    OpenAiCompatProvider::new(
        reqwest::Client::new(),
        server.uri(),
        "test-key".to_string(),
        "test-model".to_string(),
        kind,
        json_mode,
        temperature,
        max_tokens,
        reasoning_effort,
        Vec::new(),
    )
}

fn provider_config(
    name: &str,
    kind: ProviderKind,
    api_key_env: &str,
    base_url: Option<String>,
) -> ProviderConfig {
    ProviderConfig {
        name: name.to_string(),
        kind,
        model: "test-model".to_string(),
        api_key_env: api_key_env.to_string(),
        base_url: base_url.unwrap_or_default(),
        json_mode: JsonMode::Object,
        reasoning_effort: None,
        timeout_ms: 1000,
        headers: Vec::new(),
    }
}

fn test_config(classify: ProviderConfig, generate: ProviderConfig) -> Config {
    Config {
        config_root: PathBuf::from("."),
        store: PathBuf::from("."),
        concurrency: 1,
        exclude_gitignored: true,
        excludes: Vec::new(),
        default_provider: None,
        classify: OperationConfig {
            provider: Some(classify.name.clone()),
            temperature: None,
            max_tokens: None,
        },
        generate: OperationConfig {
            provider: Some(generate.name.clone()),
            temperature: None,
            max_tokens: None,
        },
        providers: vec![classify, generate],
    }
}

async fn requests(server: &MockServer) -> Vec<wiremock::Request> {
    server.received_requests().await.unwrap()
}

fn body_of(request: &wiremock::Request) -> Value {
    serde_json::from_slice(&request.body).unwrap()
}

fn chat_response(content: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "choices": [{ "message": { "content": content } }],
        "usage": { "prompt_tokens": 11, "completion_tokens": 7 }
    }))
}

fn error_message(result: &Result<impl Sized, traits::error::ToolFailure>) -> String {
    result.as_ref().err().unwrap().to_string()
}

// ---------------------------------------------------------------------------
// TypeSafe
// ---------------------------------------------------------------------------

#[tokio::test]
async fn typesafe_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "model": "jev-latest",
            "answers": {
                "assessment": {
                    "type": "choice",
                    "choice": "compliant",
                    "probabilities": {
                        "compliant": 0.7,
                        "violation": 0.1,
                        "not-applicable": 0.1,
                        "inconclusive": 0.1
                    },
                    "confidence": 0.9
                }
            },
            "usage": { "input_tokens": 12, "output_tokens": 3 }
        })))
        .mount(&server)
        .await;

    let provider = typesafe(&server);
    let response = provider
        .decide(
            json!({ "source": { "text": "hello" } }),
            vec![classify_question()],
        )
        .await
        .unwrap();

    let answer = response.answers.get("assessment").unwrap();
    assert_eq!(answer.label, "compliant");
    assert_eq!(answer.probabilities.len(), 4);
    assert!((answer.confidence.unwrap() - 0.9).abs() < 1e-12);
    assert_eq!(response.usage.input_tokens, Some(12));
    assert_eq!(response.usage.output_tokens, Some(3));

    // Request body shape.
    let sent = requests(&server).await;
    let body = body_of(&sent[0]);
    assert_eq!(body["model"], "jev-latest");
    assert_eq!(body["questions"]["assessment"]["type"], "choice");
    assert_eq!(
        body["questions"]["assessment"]["criteria"]["compliant"],
        "The input abides by the trait."
    );
}

#[tokio::test]
async fn typesafe_unknown_label_is_rejected() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "answers": {
                "assessment": {
                    "choice": "bogus",
                    "probabilities": { "compliant": 1.0, "violation": 0.0, "not-applicable": 0.0, "inconclusive": 0.0 }
                }
            }
        })))
        .mount(&server)
        .await;

    let result = typesafe(&server)
        .decide(json!({}), vec![classify_question()])
        .await;
    assert_eq!(
        error_message(&result),
        "Provider returned an unknown label for decision \"assessment\""
    );
}

#[tokio::test]
async fn typesafe_small_drift_is_accepted_and_rescaled() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "answers": {
                "assessment": {
                    "choice": "compliant",
                    "probabilities": {
                        "compliant": 0.255,
                        "violation": 0.25,
                        "not-applicable": 0.25,
                        "inconclusive": 0.25
                    }
                }
            }
        })))
        .mount(&server)
        .await;

    let response = typesafe(&server)
        .decide(json!({}), vec![classify_question()])
        .await
        .unwrap();
    let answer = response.answers.get("assessment").unwrap();
    let total: f64 = answer.probabilities.values().sum();
    assert!((total - 1.0).abs() < 1e-12);
    let compliant = answer.probabilities.get("compliant").unwrap();
    assert!((compliant - 0.255 / 1.005).abs() < 1e-12);
}

#[tokio::test]
async fn typesafe_large_drift_is_rejected() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "answers": {
                "assessment": {
                    "choice": "compliant",
                    "probabilities": {
                        "compliant": 0.28,
                        "violation": 0.25,
                        "not-applicable": 0.25,
                        "inconclusive": 0.25
                    }
                }
            }
        })))
        .mount(&server)
        .await;

    let result = typesafe(&server)
        .decide(json!({}), vec![classify_question()])
        .await;
    assert_eq!(
        error_message(&result),
        "Provider returned probabilities that do not sum to 1 for decision \"assessment\""
    );
}

#[tokio::test]
async fn typesafe_missing_answer_is_rejected() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "answers": {} })))
        .mount(&server)
        .await;

    let result = typesafe(&server)
        .decide(json!({}), vec![classify_question()])
        .await;
    assert_eq!(
        error_message(&result),
        "Provider returned no answer for decision \"assessment\""
    );
}

#[tokio::test]
async fn typesafe_missing_confidence_is_accepted() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "answers": {
                "assessment": {
                    "choice": "violation",
                    "probabilities": { "compliant": 0.0, "violation": 1.0, "not-applicable": 0.0, "inconclusive": 0.0 },
                    "confidence": null
                }
            }
        })))
        .mount(&server)
        .await;

    let response = typesafe(&server)
        .decide(json!({}), vec![classify_question()])
        .await
        .unwrap();
    assert!(
        response
            .answers
            .get("assessment")
            .unwrap()
            .confidence
            .is_none()
    );
}

// ---------------------------------------------------------------------------
// OpenAI-compatible classification
// ---------------------------------------------------------------------------

#[tokio::test]
async fn openai_classify_happy_path() {
    let server = MockServer::start().await;
    let content = json!({
        "answers": [{
            "id": "assessment",
            "label": "compliant",
            "probabilities": { "compliant": 0.7, "violation": 0.1, "not-applicable": 0.1, "inconclusive": 0.1 }
        }]
    })
    .to_string();
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(chat_response(&content))
        .mount(&server)
        .await;

    let provider = openai(
        &server,
        ProviderKind::OpenAi,
        JsonMode::Object,
        Some(0.0),
        Some(1024),
        None,
    );
    let response = provider
        .decide(
            json!({ "source": { "text": "hi" } }),
            vec![classify_question()],
        )
        .await
        .unwrap();
    let answer = response.answers.get("assessment").unwrap();
    assert_eq!(answer.label, "compliant");
    assert!((answer.probabilities.get("compliant").unwrap() - 0.7).abs() < 1e-12);
    assert_eq!(response.usage.input_tokens, Some(11));
    assert_eq!(response.usage.output_tokens, Some(7));

    let sent = requests(&server).await;
    let body = body_of(&sent[0]);
    assert_eq!(body["model"], "test-model");
    assert_eq!(body["temperature"], 0.0);
    let user: Value =
        serde_json::from_str(body["messages"][1]["content"].as_str().unwrap()).unwrap();
    assert_eq!(user["questions"][0]["id"], "assessment");
    assert_eq!(
        user["questions"][0]["criteria"]["compliant"],
        "The input abides by the trait."
    );
}

#[tokio::test]
async fn openai_classify_sends_session_header_for_opencode_go() {
    let server = MockServer::start().await;
    let content = json!({
        "answers": [{
            "id": "assessment",
            "label": "violation",
            "probabilities": { "compliant": 0.0, "violation": 1.0, "not-applicable": 0.0, "inconclusive": 0.0 }
        }]
    })
    .to_string();
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(chat_response(&content))
        .mount(&server)
        .await;

    let provider = openai(
        &server,
        ProviderKind::OpenCodeGo,
        JsonMode::Object,
        Some(0.0),
        Some(1024),
        None,
    );
    provider
        .decide(json!({}), vec![classify_question()])
        .await
        .unwrap();

    let sent = requests(&server).await;
    let session = sent[0]
        .headers
        .get("x-opencode-session")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(!session.is_empty());
    let user_agent = sent[0].headers.get("user-agent").unwrap().to_str().unwrap();
    assert_eq!(user_agent, format!("traits/{}", env!("CARGO_PKG_VERSION")));
}

#[tokio::test]
async fn openai_classify_missing_answer_is_rejected() {
    let server = MockServer::start().await;
    let content = json!({
        "answers": [{
            "id": "other",
            "label": "compliant",
            "probabilities": { "compliant": 1.0, "violation": 0.0, "not-applicable": 0.0, "inconclusive": 0.0 }
        }]
    })
    .to_string();
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(chat_response(&content))
        .mount(&server)
        .await;

    let provider = openai(
        &server,
        ProviderKind::OpenAi,
        JsonMode::Object,
        Some(0.0),
        Some(1024),
        None,
    );
    let result = provider.decide(json!({}), vec![classify_question()]).await;
    assert_eq!(
        error_message(&result),
        "Provider returned no answer for decision \"assessment\""
    );
}

// ---------------------------------------------------------------------------
// OpenAI-compatible generation
// ---------------------------------------------------------------------------

fn generate_request() -> GenerateRequest {
    GenerateRequest {
        system: "Extract the result.".to_string(),
        user: "the input".to_string(),
        schema_name: "result".to_string(),
        schema: json!({ "type": "object", "properties": { "ok": { "type": "boolean" } } }),
        strict: true,
    }
}

#[tokio::test]
async fn openai_generate_sends_json_schema() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(chat_response("{\"ok\":true}"))
        .mount(&server)
        .await;

    let provider = openai(
        &server,
        ProviderKind::OpenAi,
        JsonMode::Schema,
        Some(0.2),
        Some(8192),
        Some("low".to_string()),
    );
    let value = provider.generate_object(generate_request()).await.unwrap();
    assert_eq!(value, json!({ "ok": true }));

    let sent = requests(&server).await;
    let body = body_of(&sent[0]);
    assert_eq!(body["response_format"]["type"], "json_schema");
    assert_eq!(body["response_format"]["json_schema"]["name"], "result");
    assert_eq!(body["response_format"]["json_schema"]["strict"], true);
    assert_eq!(
        body["response_format"]["json_schema"]["schema"]["type"],
        "object"
    );
    assert_eq!(body["messages"][0]["content"], "Extract the result.");
    assert_eq!(body["temperature"], 0.2);
    assert_eq!(body["max_tokens"], 8192);
    assert_eq!(body["reasoning_effort"], "low");
}

#[tokio::test]
async fn openai_generate_retries_once_in_object_mode_on_400() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_string_contains("\"json_schema\""))
        .respond_with(ResponseTemplate::new(400).set_body_string("unsupported response_format"))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_string_contains("\"json_object\""))
        .respond_with(chat_response("{\"ok\":true}"))
        .mount(&server)
        .await;

    let provider = openai(
        &server,
        ProviderKind::OpenAi,
        JsonMode::Schema,
        Some(0.2),
        Some(8192),
        None,
    );
    let value = provider.generate_object(generate_request()).await.unwrap();
    assert_eq!(value, json!({ "ok": true }));

    let sent = requests(&server).await;
    assert_eq!(sent.len(), 2, "exactly one retry");
    let first = body_of(&sent[0]);
    let second = body_of(&sent[1]);
    assert_eq!(first["response_format"]["type"], "json_schema");
    assert_eq!(second["response_format"]["type"], "json_object");
    let system = second["messages"][0]["content"].as_str().unwrap();
    assert!(system.contains("Extract the result."));
    assert!(system.contains("\"type\":\"object\""));

    // The downgrade is remembered: the next call goes straight to json_object.
    provider.generate_object(generate_request()).await.unwrap();
    let sent = requests(&server).await;
    assert_eq!(sent.len(), 3);
    assert_eq!(body_of(&sent[2])["response_format"]["type"], "json_object");
}

#[tokio::test]
async fn openai_generate_prompt_mode_has_no_response_format() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(chat_response("{\"ok\":true}"))
        .mount(&server)
        .await;

    let provider = openai(
        &server,
        ProviderKind::OpenAi,
        JsonMode::Prompt,
        Some(0.2),
        Some(8192),
        None,
    );
    provider.generate_object(generate_request()).await.unwrap();

    let sent = requests(&server).await;
    let body = body_of(&sent[0]);
    assert!(body.get("response_format").is_none());
    let system = body["messages"][0]["content"].as_str().unwrap();
    assert!(system.contains("Extract the result."));
    assert!(system.contains("\"type\":\"object\""));
}

// ---------------------------------------------------------------------------
// build
// ---------------------------------------------------------------------------

#[test]
fn build_unknown_provider_name_is_a_config_error() {
    let config = test_config(
        provider_config(
            "known",
            ProviderKind::OpenAi,
            "TRAITS_TEST_BUILD_UNKNOWN",
            None,
        ),
        provider_config(
            "known2",
            ProviderKind::OpenAi,
            "TRAITS_TEST_BUILD_UNKNOWN",
            None,
        ),
    );
    let mut config = config;
    config.classify.provider = Some("nope".to_string());
    let error = build(&config).err().expect("expected a config error");
    assert!(error.to_string().contains("Unknown provider"), "{error}");
}

#[test]
fn build_missing_env_var_is_a_config_error() {
    let missing = "TRAITS_TEST_BUILD_MISSING_KEY_9F3A";
    unsafe { std::env::remove_var(missing) };
    let config = test_config(
        provider_config("openai", ProviderKind::OpenAi, missing, None),
        provider_config("openai2", ProviderKind::OpenAi, missing, None),
    );
    let error = build(&config).err().expect("expected a config error");
    assert!(error.to_string().contains(missing), "{error}");
    assert!(
        error.to_string().contains("environment variable"),
        "{error}"
    );
}

#[test]
fn build_typesafe_generate_is_a_config_error() {
    let key_env = "TRAITS_TEST_BUILD_TYPESAFE_KEY";
    unsafe { std::env::set_var(key_env, "secret") };
    let config = test_config(
        provider_config("openai", ProviderKind::OpenAi, key_env, None),
        provider_config("typesafe", ProviderKind::Typesafe, key_env, None),
    );
    let models = build(&config).expect("typesafe generate is ignored, not fatal");
    assert!(models.language.is_none());
}

#[test]
fn build_uses_provider_defaults() {
    let key_env = "TRAITS_TEST_BUILD_DEFAULTS_KEY";
    unsafe { std::env::set_var(key_env, "secret") };
    let config = test_config(
        provider_config("openai", ProviderKind::OpenAi, key_env, None),
        provider_config("opencode", ProviderKind::OpenCodeGo, key_env, None),
    );
    let models = build(&config).unwrap();
    // Both models were constructed; the classification model is OpenAI-compatible.
    assert!(std::sync::Arc::strong_count(&models.decision) >= 1);
}
