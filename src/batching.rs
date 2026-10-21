//! Assessment planning and the `check_traits` orchestration.
//!
//! Each trait's question is rendered once. Files split into 200-line chunks,
//! each chunk looks up its active traits in the cache and packs the misses into
//! byte-budgeted batches, and every batch runs through one bounded, ordered
//! stream that stops at the first failure.

use std::num::NonZeroUsize;
use std::ops::Range;
use std::path::Path;

use futures::{StreamExt, TryStreamExt, stream};
use serde_json::{Map, Value, json};

use crate::cache::{self, Key, Keys};
use crate::config::Config;
use crate::doc::{
    Assessment, Classification, Probabilities, Source, SourceLocation, Trait, TraitEvaluation,
    TraitName,
};
use crate::error::ToolFailure;
use crate::files;
use crate::prompt::assessment_instructions;
use crate::provider::{self, Answer, Choice, Models, Question};
use crate::store::TraitStore;

/// Decision request budget shared by the source and every packed question.
const MAX_BYTES: usize = 24_000;
/// Maximum number of traits per decision request.
const BATCH_LIMIT: usize = 16;
/// Number of source lines assessed per chunk.
const CHUNK_LINES: usize = 200;

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
            let mut selected: Vec<Trait> = Vec::new();
            for name in names {
                if !selected.iter().any(|trait_value| &trait_value.name == name) {
                    selected.push(store.get(name).await?);
                }
            }
            selected
        }
        None => store.ls().await?,
    };

    let paths = files::resolve_includes(config, include)?
        .into_iter()
        .filter(|path| git_files.as_ref().is_none_or(|files| files.contains(path)))
        .collect::<Vec<_>>();
    tracing::debug!(
        traits = traits.len(),
        files = paths.len(),
        "check_traits resolved selection"
    );

    let questions = Questions::new(&traits)?;
    let provider = match &config.classify.provider {
        Some(provider) => json!({
            "kind": provider.kind.as_str(),
            "model": provider.model,
            "baseUrl": provider.base_url,
        }),
        None => Value::Null,
    };
    let mut plan = Plan::new(circumstances, &provider, config.cache_dir.as_deref())?;
    for path in &paths {
        let text = tokio::fs::read_to_string(path).await.map_err(|error| {
            ToolFailure::platform(format!("failed to read {}: {error}", path.display()))
        })?;
        let active = files::exclude_ignored(&traits, &text);
        let display = path.to_string_lossy();
        for (start_line, excerpt) in chunks(&text) {
            let source = Source {
                text: excerpt.to_string(),
                location: Some(SourceLocation::File {
                    path: display.to_string(),
                    start_line,
                }),
            };
            plan.add(&questions, source, &active)?;
        }
    }

    tracing::debug!(
        pairs = plan.pairs.len(),
        cached = plan
            .pairs
            .iter()
            .filter(|pair| pair.cached.is_some())
            .count(),
        requests = plan.batches.len(),
        "check_traits planned"
    );
    let verdicts = plan.assess(models, &questions, config.concurrency).await?;
    Ok(verdicts
        .iter()
        .filter(|verdict| verdict.assessment.classification.requires_review())
        .map(|verdict| plan.evaluation(&questions, verdict))
        .collect())
}

/// Split text into runs of `CHUNK_LINES` newline-separated lines, numbering
/// each run by its first line. Empty text is one empty chunk.
fn chunks(text: &str) -> impl Iterator<Item = (u32, &str)> {
    let ends = text
        .match_indices('\n')
        .map(|(index, _)| index)
        .chain([text.len()])
        .collect::<Vec<_>>();
    (0..ends.len()).step_by(CHUNK_LINES).map(move |first| {
        let start = if first == 0 { 0 } else { ends[first - 1] + 1 };
        let last = (first + CHUNK_LINES).min(ends.len()) - 1;
        (first as u32 + 1, &text[start..ends[last]])
    })
}

/// The selected traits, each with its assessment question rendered and
/// encoded once.
struct Questions<'a> {
    traits: &'a [Trait],
    instructions: Vec<String>,
    encoded: Vec<Vec<u8>>,
}

impl<'a> Questions<'a> {
    fn new(traits: &'a [Trait]) -> Result<Self, ToolFailure> {
        let instructions = traits
            .iter()
            .map(assessment_instructions)
            .collect::<Vec<_>>();
        let encoded = traits
            .iter()
            .zip(&instructions)
            .map(|(trait_value, instructions)| {
                encode(&question_json(trait_value.name.as_str(), instructions))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            traits,
            instructions,
            encoded,
        })
    }

    fn question(&self, index: usize) -> Question<'_> {
        Question {
            id: self.traits[index].name.as_str(),
            instructions: &self.instructions[index],
            criteria: Classification::CRITERIA,
        }
    }
}

/// One decision request: a source and a run of `Plan::members`.
struct Batch {
    source: usize,
    members: Range<usize>,
}

/// One trait's question about one source, with its cached assessment when the
/// cache has one.
struct Pair {
    source: usize,
    trait_index: usize,
    key: Key,
    cached: Option<Assessment>,
}

/// One trait's assessment of one source.
struct Verdict {
    source: usize,
    trait_index: usize,
    assessment: Assessment,
}

/// Every source excerpt, its trait pairs in finding order, and the batches
/// that ask the uncached pairs. Batches index `members`, which index `pairs`.
#[derive(Default)]
struct Plan<'a> {
    circumstances: Option<&'a str>,
    provider: Vec<u8>,
    cache_dir: Option<&'a Path>,
    sources: Vec<Source>,
    pairs: Vec<Pair>,
    members: Vec<usize>,
    batches: Vec<Batch>,
}

impl<'a> Plan<'a> {
    fn new(
        circumstances: Option<&'a str>,
        provider: &Value,
        cache_dir: Option<&'a Path>,
    ) -> Result<Self, ToolFailure> {
        Ok(Self {
            circumstances,
            provider: encode(provider)?,
            cache_dir,
            ..Self::default()
        })
    }

    /// Look up the `active` trait indices that assess `source` in the cache,
    /// then pack the misses into batches that fit the request budget.
    fn add(
        &mut self,
        questions: &Questions<'_>,
        source: Source,
        active: &[usize],
    ) -> Result<(), ToolFailure> {
        if active.is_empty() {
            return Ok(());
        }
        let input = encode(&input_json(&source, self.circumstances))?;
        let keys = Keys::new(&self.provider, &input);
        let input_bytes = input.len();
        let index = self.sources.len();
        let mut start = self.members.len();
        let mut bytes = input_bytes;
        for &trait_index in active {
            let key = keys.key(&questions.encoded[trait_index]);
            let cached = self.cache_dir.and_then(|dir| cache::read(dir, key));
            let pair = self.pairs.len();
            let hit = cached.is_some();
            self.pairs.push(Pair {
                source: index,
                trait_index,
                key,
                cached,
            });
            if hit {
                continue;
            }
            let question_bytes = questions.encoded[trait_index].len();
            if input_bytes + question_bytes > MAX_BYTES {
                return Err(ToolFailure::too_large(
                    questions.traits[trait_index].name.as_str(),
                    input_bytes + question_bytes,
                    MAX_BYTES,
                    source.location.clone(),
                ));
            }
            let packed = self.members.len() - start;
            if packed > 0 && (packed == BATCH_LIMIT || bytes + question_bytes > MAX_BYTES) {
                self.batches.push(Batch {
                    source: index,
                    members: start..self.members.len(),
                });
                start = self.members.len();
                bytes = input_bytes;
            }
            self.members.push(pair);
            bytes += question_bytes;
        }
        if self.members.len() > start {
            self.batches.push(Batch {
                source: index,
                members: start..self.members.len(),
            });
        }
        self.sources.push(source);
        Ok(())
    }

    /// Run every batch, at most `concurrency` at a time, in plan order, and
    /// merge the answers with the cached pairs. The first failure drops the
    /// requests still in flight and starts no more.
    async fn assess(
        &self,
        models: &Models,
        questions: &Questions<'_>,
        concurrency: NonZeroUsize,
    ) -> Result<Vec<Verdict>, ToolFailure> {
        let requests = (0..self.batches.len()).map(|batch| self.decide(models, questions, batch));
        let answers: Vec<Vec<Assessment>> = stream::iter(requests)
            .buffered(concurrency.get())
            .try_collect()
            .await?;

        let mut fresh = answers.into_iter().flatten();
        Ok(self
            .pairs
            .iter()
            .map(|pair| Verdict {
                source: pair.source,
                trait_index: pair.trait_index,
                assessment: pair
                    .cached
                    .clone()
                    .or_else(|| fresh.next())
                    .expect("batches answer every uncached pair in order"),
            })
            .collect())
    }

    /// Ask one batch, caching each conclusive assessment as soon as it
    /// arrives so a later failure keeps it.
    async fn decide(
        &self,
        models: &Models,
        questions: &Questions<'_>,
        batch: usize,
    ) -> Result<Vec<Assessment>, ToolFailure> {
        let batch = &self.batches[batch];
        let members = &self.members[batch.members.clone()];
        let asked = members
            .iter()
            .map(|&pair| questions.question(self.pairs[pair].trait_index))
            .collect::<Vec<_>>();
        let input = input_json(&self.sources[batch.source], self.circumstances);
        let assessments = provider::decide(models.decision.as_ref(), &input, &asked)
            .await?
            .iter()
            .map(assessment)
            .collect::<Vec<_>>();
        if let Some(dir) = self.cache_dir {
            for (&pair, assessment) in members.iter().zip(&assessments) {
                if assessment.classification != Classification::Inconclusive {
                    cache::write(dir, self.pairs[pair].key, assessment).await;
                }
            }
        }
        Ok(assessments)
    }

    fn evaluation(&self, questions: &Questions<'_>, verdict: &Verdict) -> TraitEvaluation {
        TraitEvaluation {
            r#trait: questions.traits[verdict.trait_index].clone(),
            source: self.sources[verdict.source].clone(),
            assessment: verdict.assessment.clone(),
        }
    }
}

fn assessment(answer: &Answer) -> Assessment {
    let probability =
        |classification: Classification| answer.probabilities[classification as usize];
    Assessment {
        classification: Classification::ALL[answer.choice],
        probabilities: Probabilities {
            compliant: probability(Classification::Compliant),
            violation: probability(Classification::Violation),
            not_applicable: probability(Classification::NotApplicable),
            inconclusive: probability(Classification::Inconclusive),
        },
    }
}

fn input_json(source: &Source, circumstances: Option<&str>) -> Value {
    let mut map = Map::new();
    map.insert(
        "source".to_string(),
        serde_json::to_value(source).expect("Source serializes"),
    );
    if let Some(circumstances) = circumstances {
        map.insert(
            "circumstances".to_string(),
            Value::String(circumstances.to_string()),
        );
    }
    Value::Object(map)
}

/// The question as a provider receives it, measured against the request budget.
fn question_json(name: &str, instructions: &str) -> Value {
    let criteria = Classification::CRITERIA
        .iter()
        .map(|criterion| {
            (
                criterion.label.to_string(),
                Value::String(criterion.description.to_string()),
            )
        })
        .collect::<Map<_, _>>();
    json!({ name: { "instructions": instructions, "criteria": criteria } })
}

fn encode(value: &Value) -> Result<Vec<u8>, ToolFailure> {
    serde_json::to_vec(value).map_err(|error| {
        ToolFailure::platform(format!("failed to encode decision request: {error}"))
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use super::*;
    use crate::provider::{DecisionModel, DecisionResponse, Usage};

    #[derive(Default)]
    struct MockDecision {
        calls: Mutex<Vec<Vec<String>>>,
        omitted: bool,
        truncated_probabilities: bool,
        violation: Vec<String>,
    }

    #[async_trait]
    impl DecisionModel for MockDecision {
        async fn decide(
            &self,
            _state: &Value,
            questions: &[Question<'_>],
        ) -> Result<DecisionResponse, ToolFailure> {
            self.calls.lock().unwrap().push(
                questions
                    .iter()
                    .map(|question| question.id.to_string())
                    .collect(),
            );
            let answers = if self.omitted {
                Vec::new()
            } else {
                questions
                    .iter()
                    .map(|question| {
                        let label = if self.violation.iter().any(|name| name == question.id) {
                            Classification::Violation
                        } else {
                            Classification::Compliant
                        };
                        let mut probabilities = vec![0.0; Classification::ALL.len()];
                        probabilities[label as usize] = 1.0;
                        if self.truncated_probabilities {
                            probabilities.truncate(1);
                        }
                        Answer {
                            choice: label as usize,
                            probabilities,
                            confidence: None,
                        }
                    })
                    .collect()
            };
            Ok(DecisionResponse {
                answers,
                usage: Usage::default(),
            })
        }
    }

    fn models(mock: &Arc<MockDecision>) -> Models {
        let decision: Arc<dyn DecisionModel> = mock.clone();
        Models {
            decision,
            language: None,
        }
    }

    fn trait_named(name: &str) -> Trait {
        Trait {
            name: TraitName::parse(name).unwrap(),
            subject: "Binding declarations.".to_string(),
            requirement: "Prefer const declarations.".to_string(),
            applicability: Some("In TypeScript, except when mutation is required.".to_string()),
            purpose: "Make immutable bindings explicit.".to_string(),
            guidance: None,
            context: None,
        }
    }

    fn generated_source(text: &str) -> Source {
        Source {
            text: text.to_string(),
            location: Some(SourceLocation::Generated {}),
        }
    }

    /// Assess every trait against one source, returning every evaluation.
    async fn evaluate_all(
        models: &Models,
        traits: &[Trait],
        source: Source,
    ) -> Result<Vec<TraitEvaluation>, ToolFailure> {
        let questions = Questions::new(traits)?;
        let mut plan = Plan::new(None, &Value::Null, None)?;
        let active = (0..traits.len()).collect::<Vec<_>>();
        plan.add(&questions, source, &active)?;
        let verdicts = plan
            .assess(models, &questions, NonZeroUsize::new(4).unwrap())
            .await?;
        Ok(verdicts
            .iter()
            .map(|verdict| plan.evaluation(&questions, verdict))
            .collect())
    }

    #[tokio::test]
    async fn splits_into_batches_of_sixteen_and_four() {
        let traits: Vec<Trait> = (0..20)
            .map(|index| trait_named(&format!("example-trait-{index}")))
            .collect();
        let mock = Arc::new(MockDecision {
            violation: vec!["example-trait-3".to_string()],
            ..Default::default()
        });
        let evaluations =
            evaluate_all(&models(&mock), &traits, generated_source("const value = 1"))
                .await
                .unwrap();

        let names = traits
            .iter()
            .map(|entry| entry.name.as_str().to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            evaluations
                .iter()
                .map(|entry| entry.r#trait.name.as_str().to_string())
                .collect::<Vec<_>>(),
            names
        );
        assert_eq!(
            evaluations[3].assessment.classification,
            Classification::Violation,
            "answers stay attached to their own trait"
        );
        let calls = mock.calls.lock().unwrap();
        // The fixed assessment prompt is ~1.7 KB, so the 24_000-byte budget
        // flushes well before the 16-trait cap.
        assert_eq!(calls.len(), 2);
        assert!(calls.iter().all(|batch| (1..=16).contains(&batch.len())));
        assert_eq!(calls.iter().flatten().cloned().collect::<Vec<_>>(), names);
    }

    #[tokio::test]
    async fn fails_oversized_sources_before_calling_the_model() {
        let traits = vec![trait_named("example-trait-0")];
        let mock = Arc::new(MockDecision::default());
        let error = evaluate_all(
            &models(&mock),
            &traits,
            generated_source(&"x".repeat(MAX_BYTES)),
        )
        .await
        .unwrap_err();

        assert!(mock.calls.lock().unwrap().is_empty());
        match error {
            ToolFailure::TraitAssessmentTooLargeError {
                trait_name,
                bytes,
                max_bytes,
                location,
                ..
            } => {
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
        let evaluations = evaluate_all(&models(&mock), &[], generated_source("const value = 1"))
            .await
            .unwrap();
        assert!(evaluations.is_empty());
        assert!(mock.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn fails_missing_answers() {
        let mock = Arc::new(MockDecision {
            omitted: true,
            ..Default::default()
        });
        let traits = vec![
            trait_named("example-trait-0"),
            trait_named("example-trait-1"),
        ];
        let error = evaluate_all(&models(&mock), &traits, generated_source("const value = 1"))
            .await
            .unwrap_err();
        assert!(
            matches!(error, ToolFailure::AiError { .. }),
            "got {error:?}"
        );
    }

    #[tokio::test]
    async fn fails_malformed_answers_instead_of_panicking() {
        let mock = Arc::new(MockDecision {
            truncated_probabilities: true,
            ..Default::default()
        });
        let traits = vec![trait_named("example-trait-0")];
        let error = evaluate_all(&models(&mock), &traits, generated_source("const value = 1"))
            .await
            .unwrap_err();
        assert!(
            matches!(error, ToolFailure::AiError { .. }),
            "got {error:?}"
        );
    }

    fn joined_chunks(text: &str) -> Vec<(u32, String)> {
        let lines: Vec<&str> = text.split('\n').collect();
        lines
            .chunks(CHUNK_LINES)
            .enumerate()
            .map(|(index, chunk)| ((index * CHUNK_LINES) as u32 + 1, chunk.join("\n")))
            .collect()
    }

    #[test]
    fn chunks_slice_exactly_what_splitting_and_joining_lines_produced() {
        let numbered = |count: usize| {
            (1..=count)
                .map(|index| format!("line {index}"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        for text in [
            String::new(),
            "\n".to_string(),
            "one line".to_string(),
            numbered(199),
            numbered(200),
            numbered(201),
            format!("{}\n", numbered(200)),
            numbered(450).replace('\n', "\r\n"),
            "\n\n\n".repeat(150),
        ] {
            let sliced = chunks(&text)
                .map(|(start, excerpt)| (start, excerpt.to_string()))
                .collect::<Vec<_>>();
            assert_eq!(sliced, joined_chunks(&text), "{text:?}");
        }
    }
}
