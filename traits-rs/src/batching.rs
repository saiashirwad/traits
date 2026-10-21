//! Assessment batching and the `check_traits` orchestration.
//!
//! Ports `evaluateAll` from `TraitEvaluation.ts` and the `check_traits` handler
//! from `toolkit/CheckTraits.ts`.

use std::collections::HashSet;

use futures::future::join_all;
use serde_json::{Map, Value};
use tokio::sync::Semaphore;

use crate::config::Config;
use crate::doc::{Assessment, Classification, Probabilities, Source, SourceLocation, Trait, TraitEvaluation, TraitName};
use crate::error::ToolFailure;
use crate::files;
use crate::prompt::assessment_question;
use crate::provider::{ClassifyAnswer, ClassifyQuestion, Models};
use crate::store::TraitStore;

/// Decision request budget shared by the source and every packed question.
const MAX_BYTES: usize = 24_000;
/// Maximum number of traits per decision request.
const BATCH_LIMIT: usize = 16;
/// Number of source lines assessed per chunk.
const CHUNK_LINES: usize = 200;

/// Assess every trait against one source excerpt, packing traits into
/// name-keyed decision batches.
pub async fn evaluate_all(
    models: &Models,
    traits: &[Trait],
    source: Source,
    circumstances: Option<&str>,
    concurrency: usize,
) -> Result<Vec<TraitEvaluation>, ToolFailure> {
    if traits.is_empty() {
        return Ok(Vec::new());
    }

    let input = input_json(&source, circumstances);
    let input_bytes = serialized_len(&input)?;

    struct Entry {
        trait_value: Trait,
        question: ClassifyQuestion,
    }

    let mut batches: Vec<Vec<Entry>> = Vec::new();
    let mut batch: Vec<Entry> = Vec::new();
    let mut bytes = input_bytes;
    for trait_value in traits {
        let question = assessment_question(trait_value);
        let question_bytes = serialized_len(&question_json(trait_value.name.as_str(), &question))?;
        if input_bytes + question_bytes > MAX_BYTES {
            return Err(ToolFailure::too_large(
                trait_value.name.as_str(),
                input_bytes + question_bytes,
                MAX_BYTES,
                source.location.clone(),
            ));
        }
        if !batch.is_empty() && (batch.len() == BATCH_LIMIT || bytes + question_bytes > MAX_BYTES) {
            batches.push(std::mem::take(&mut batch));
            bytes = input_bytes;
        }
        batch.push(Entry { trait_value: trait_value.clone(), question });
        bytes += question_bytes;
    }
    if !batch.is_empty() {
        batches.push(batch);
    }

    let permits = concurrency.max(1);
    let semaphore = Semaphore::new(permits);
    let futures = batches.iter().map(|entries| {
        let semaphore = &semaphore;
        let input = &input;
        let source = source.clone();
        async move {
            let _permit = semaphore.acquire().await.map_err(|_| ToolFailure::ai("assessment semaphore closed"))?;
            let questions: Vec<(String, ClassifyQuestion)> = entries
                .iter()
                .map(|entry| (entry.trait_value.name.0.clone(), entry.question.clone()))
                .collect();
            let response = models.decision.decide(input.clone(), questions).await?;
            let mut evaluations = Vec::with_capacity(entries.len());
            for entry in entries {
                let name = entry.trait_value.name.as_str();
                let answer = response
                    .answers
                    .get(name)
                    .ok_or_else(|| ToolFailure::ai(format!("Decision model returned no answer for trait {name}")))?;
                evaluations.push(TraitEvaluation {
                    r#trait: entry.trait_value.clone(),
                    source: source.clone(),
                    assessment: assessment_from(answer)?,
                });
            }
            Ok::<_, ToolFailure>(evaluations)
        }
    });
    let results = join_all(futures).await;
    let mut evaluations = Vec::new();
    for result in results {
        evaluations.extend(result?);
    }
    Ok(evaluations)
}

/// Discover included files, select traits, and assess each 200-line chunk.
pub async fn check_traits(
    config: &Config,
    store: &dyn TraitStore,
    models: &Models,
    include: &[String],
    trait_names: Option<&[TraitName]>,
    circumstances: Option<&str>,
) -> Result<Vec<TraitEvaluation>, ToolFailure> {
    let git_files = files::git_included_files(config).await?;

    let traits = match trait_names {
        Some(names) => {
            let mut seen = HashSet::new();
            let mut selected = Vec::new();
            for name in names {
                if seen.insert(name.clone()) {
                    selected.push(store.get(name).await?);
                }
            }
            selected
        }
        None => store.ls().await?,
    };

    let mut paths = Vec::new();
    let mut seen_paths = HashSet::new();
    for candidate in files::resolve_includes(config, include)? {
        let absolute = files::absolute(&config.config_root, &candidate);
        if !seen_paths.insert(absolute.clone()) {
            continue;
        }
        if let Some(git_files) = &git_files
            && !git_files.contains(&absolute)
        {
            continue;
        }
        if let Ok(metadata) = tokio::fs::metadata(&absolute).await
            && metadata.is_file()
        {
            paths.push(absolute);
        }
    }

    struct Work {
        traits: Vec<Trait>,
        source: Source,
    }

    let mut work: Vec<Work> = Vec::new();
    for path in &paths {
        let text = tokio::fs::read_to_string(path)
            .await
            .map_err(|error| ToolFailure::platform(format!("failed to read {}: {error}", path.display())))?;
        let active = files::exclude_ignored(&traits, &text);
        let lines: Vec<&str> = text.split('\n').collect();
        for (index, chunk) in lines.chunks(CHUNK_LINES).enumerate() {
            let start_line = (index as u32) * CHUNK_LINES as u32 + 1;
            work.push(Work {
                traits: active.clone(),
                source: Source {
                    text: chunk.join("\n"),
                    location: Some(SourceLocation::File { path: path.to_string_lossy().into_owned(), start_line }),
                },
            });
        }
    }

    let permits = config.concurrency.max(1);
    let semaphore = Semaphore::new(permits);
    let futures = work.iter().map(|item| {
        let semaphore = &semaphore;
        async move {
            let _permit = semaphore.acquire().await.map_err(|_| ToolFailure::ai("assessment semaphore closed"))?;
            evaluate_all(models, &item.traits, item.source.clone(), circumstances, permits).await
        }
    });
    let results = join_all(futures).await;

    let mut findings = Vec::new();
    for result in results {
        for evaluation in result? {
            if evaluation.requires_review() {
                findings.push(evaluation);
            }
        }
    }
    Ok(findings)
}

fn input_json(source: &Source, circumstances: Option<&str>) -> Value {
    let mut map = Map::new();
    map.insert("source".to_string(), serde_json::to_value(source).expect("Source serializes"));
    if let Some(circumstances) = circumstances {
        map.insert("circumstances".to_string(), Value::String(circumstances.to_string()));
    }
    Value::Object(map)
}

fn question_json(name: &str, question: &ClassifyQuestion) -> Value {
    let mut criteria = Map::new();
    for (key, value) in &question.criteria {
        criteria.insert(key.clone(), Value::String(value.clone()));
    }
    let mut body = Map::new();
    body.insert("instructions".to_string(), Value::String(question.instructions.clone()));
    body.insert("criteria".to_string(), Value::Object(criteria));
    let mut outer = Map::new();
    outer.insert(name.to_string(), Value::Object(body));
    Value::Object(outer)
}

fn serialized_len(value: &Value) -> Result<usize, ToolFailure> {
    serde_json::to_vec(value)
        .map(|bytes| bytes.len())
        .map_err(|error| ToolFailure::platform(format!("failed to measure decision request: {error}")))
}

fn assessment_from(answer: &ClassifyAnswer) -> Result<Assessment, ToolFailure> {
    let classification = match answer.label.as_str() {
        "compliant" => Classification::Compliant,
        "violation" => Classification::Violation,
        "not-applicable" => Classification::NotApplicable,
        "inconclusive" => Classification::Inconclusive,
        other => {
            return Err(ToolFailure::ai(format!("Decision model returned unknown classification {other:?}")));
        }
    };
    let probability = |label: &str| {
        answer
            .probabilities
            .get(label)
            .copied()
            .ok_or_else(|| ToolFailure::ai(format!("Decision model omitted probability {label:?}")))
    };
    let probabilities = Probabilities {
        compliant: probability("compliant")?,
        violation: probability("violation")?,
        not_applicable: probability("not-applicable")?,
        inconclusive: probability("inconclusive")?,
    };
    Ok(Assessment { classification, probabilities })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use indexmap::IndexMap;

    use super::*;
    use crate::doc::TraitName;
    use crate::provider::{DecisionResponse, GenerateRequest, LanguageModel, Usage};

    #[derive(Default)]
    struct MockDecision {
        calls: Mutex<Vec<Vec<String>>>,
        omitted: bool,
        violation: Vec<String>,
        inconclusive: Vec<String>,
        omit_label: Option<String>,
    }

    impl MockDecision {
        fn answer(&self, name: &str) -> ClassifyAnswer {
            let label = if self.violation.iter().any(|entry| entry == name) {
                "violation"
            } else if self.inconclusive.iter().any(|entry| entry == name) {
                "inconclusive"
            } else {
                "compliant"
            };
            let mut probabilities = IndexMap::new();
            for candidate in ["compliant", "violation", "not-applicable", "inconclusive"] {
                if Some(candidate) == self.omit_label.as_deref() {
                    continue;
                }
                probabilities.insert(candidate.to_string(), if candidate == label { 1.0 } else { 0.0 });
            }
            ClassifyAnswer { label: label.to_string(), probabilities, confidence: None }
        }
    }

    #[async_trait]
    impl crate::provider::DecisionModel for MockDecision {
        async fn decide(
            &self,
            _state: Value,
            questions: Vec<(String, ClassifyQuestion)>,
        ) -> Result<DecisionResponse, ToolFailure> {
            self.calls.lock().unwrap().push(questions.iter().map(|(name, _)| name.clone()).collect());
            let mut answers = IndexMap::new();
            if !self.omitted {
                for (name, _) in &questions {
                    answers.insert(name.clone(), self.answer(name));
                }
            }
            Ok(DecisionResponse { answers, usage: Usage::default() })
        }
    }

    struct NopLanguage;

    #[async_trait]
    impl LanguageModel for NopLanguage {
        async fn generate_object(&self, _request: GenerateRequest) -> Result<Value, ToolFailure> {
            Err(ToolFailure::platform("language model is not available in tests"))
        }
    }

    fn models(mock: &Arc<MockDecision>) -> Models {
        let decision: Arc<dyn crate::provider::DecisionModel> = mock.clone();
        Models { decision, language: Arc::new(NopLanguage) }
    }

    fn trait_named(name: &str) -> Trait {
        Trait {
            name: TraitName(name.to_string()),
            subject: "Binding declarations.".to_string(),
            requirement: "Prefer const declarations.".to_string(),
            applicability: Some("In TypeScript, except when mutation is required.".to_string()),
            purpose: "Make immutable bindings explicit.".to_string(),
            guidance: None,
            context: None,
        }
    }

    fn generated_source(text: &str) -> Source {
        Source { text: text.to_string(), location: Some(SourceLocation::Generated {}) }
    }

    #[tokio::test]
    async fn splits_into_batches_of_sixteen_and_four() {
        let traits: Vec<Trait> = (0..20).map(|index| trait_named(&format!("example-trait-{index}"))).collect();
        let mock = Arc::new(MockDecision::default());
        let evaluations = evaluate_all(&models(&mock), &traits, generated_source("const value = 1"), None, 1)
            .await
            .unwrap();

        assert_eq!(evaluations.len(), 20);
        let names: Vec<String> = evaluations.iter().map(|entry| entry.r#trait.name.0.clone()).collect();
        assert_eq!(names, traits.iter().map(|entry| entry.name.0.clone()).collect::<Vec<_>>());
        let calls = mock.calls.lock().unwrap();
        // The fixed assessment prompt is ~1.7 KB, so the 24_000-byte budget
        // flushes well before the 16-trait cap; the TS test likewise only
        // asserts at most 16 questions per batch and two batches total.
        assert_eq!(calls.len(), 2);
        assert!(calls.iter().all(|batch| (1..=16).contains(&batch.len())));
        let ordered: Vec<String> = calls.iter().flatten().cloned().collect();
        assert_eq!(ordered, traits.iter().map(|entry| entry.name.0.clone()).collect::<Vec<_>>());
    }

    #[tokio::test]
    async fn fails_oversized_sources_before_calling_the_model() {
        let traits = vec![trait_named("example-trait-0")];
        let mock = Arc::new(MockDecision::default());
        let error = evaluate_all(&models(&mock), &traits, generated_source(&"x".repeat(MAX_BYTES)), None, 4)
            .await
            .unwrap_err();

        assert!(mock.calls.lock().unwrap().is_empty());
        match error {
            ToolFailure::TraitAssessmentTooLargeError { trait_name, bytes, max_bytes, location, .. } => {
                assert_eq!(trait_name, "example-trait-0");
                assert_eq!(max_bytes, MAX_BYTES);
                assert!(bytes > max_bytes);
                assert!(matches!(location, Some(SourceLocation::Generated {})));
            }
            other => panic!("expected TraitAssessmentTooLargeError, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn empty_selection_does_not_call_the_model() {
        let mock = Arc::new(MockDecision::default());
        let evaluations = evaluate_all(&models(&mock), &[], generated_source("const value = 1"), None, 4)
            .await
            .unwrap();
        assert!(evaluations.is_empty());
        assert!(mock.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn fails_missing_answers() {
        let mock = Arc::new(MockDecision { omitted: true, ..Default::default() });
        let traits = vec![trait_named("example-trait-0"), trait_named("example-trait-1")];
        let error = evaluate_all(&models(&mock), &traits, generated_source("const value = 1"), None, 4)
            .await
            .unwrap_err();
        assert!(matches!(error, ToolFailure::AiError { .. }), "got {error:?}");
    }

    #[tokio::test]
    async fn fails_missing_probability_labels() {
        let mock = Arc::new(MockDecision {
            omit_label: Some("inconclusive".to_string()),
            ..Default::default()
        });
        let traits = vec![trait_named("example-trait-0")];
        let error = evaluate_all(&models(&mock), &traits, generated_source("const value = 1"), None, 4)
            .await
            .unwrap_err();
        assert!(matches!(error, ToolFailure::AiError { .. }), "got {error:?}");
    }
}
