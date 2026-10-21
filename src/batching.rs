//! Assessment planning and the `check_traits` orchestration.
//!
//! Each trait's question is rendered once. Files split into 200-line chunks,
//! each chunk packs its active traits into byte-budgeted batches of trait
//! indices, and every batch runs through one bounded, ordered stream that stops
//! at the first failure.

use std::num::NonZeroUsize;
use std::ops::Range;

use futures::{StreamExt, TryStreamExt, stream};
use serde_json::{Map, Value, json};

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
    let mut plan = Plan::new(circumstances);
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

/// The selected traits, each with its assessment question rendered once.
struct Questions<'a> {
    traits: &'a [Trait],
    instructions: Vec<String>,
    bytes: Vec<usize>,
}

impl<'a> Questions<'a> {
    fn new(traits: &'a [Trait]) -> Result<Self, ToolFailure> {
        let instructions = traits
            .iter()
            .map(assessment_instructions)
            .collect::<Vec<_>>();
        let bytes = traits
            .iter()
            .zip(&instructions)
            .map(|(trait_value, instructions)| {
                serialized_len(&question_json(trait_value.name.as_str(), instructions))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            traits,
            instructions,
            bytes,
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

/// One trait's assessment of one source.
struct Verdict {
    source: usize,
    trait_index: usize,
    assessment: Assessment,
}

/// Every source excerpt and the trait batches that assess it. Batches index
/// `members`, which index the trait questions.
#[derive(Default)]
struct Plan<'a> {
    circumstances: Option<&'a str>,
    sources: Vec<Source>,
    members: Vec<usize>,
    batches: Vec<Batch>,
}

impl<'a> Plan<'a> {
    fn new(circumstances: Option<&'a str>) -> Self {
        Self {
            circumstances,
            ..Self::default()
        }
    }

    /// Pack the `active` trait indices that assess `source` into batches that
    /// fit the request budget.
    fn add(
        &mut self,
        questions: &Questions<'_>,
        source: Source,
        active: &[usize],
    ) -> Result<(), ToolFailure> {
        if active.is_empty() {
            return Ok(());
        }
        let input_bytes = serialized_len(&input_json(&source, self.circumstances))?;
        let index = self.sources.len();
        let mut start = self.members.len();
        let mut bytes = input_bytes;
        for &trait_index in active {
            let question_bytes = questions.bytes[trait_index];
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
            self.members.push(trait_index);
            bytes += question_bytes;
        }
        self.batches.push(Batch {
            source: index,
            members: start..self.members.len(),
        });
        self.sources.push(source);
        Ok(())
    }

    /// Run every batch, at most `concurrency` at a time, in plan order. The
    /// first failure drops the requests still in flight and starts no more.
    async fn assess(
        &self,
        models: &Models,
        questions: &Questions<'_>,
        concurrency: NonZeroUsize,
    ) -> Result<Vec<Verdict>, ToolFailure> {
        let requests = (0..self.batches.len()).map(|batch| self.decide(models, questions, batch));
        let answers: Vec<Vec<Answer>> = stream::iter(requests)
            .buffered(concurrency.get())
            .try_collect()
            .await?;

        Ok(self
            .batches
            .iter()
            .zip(answers)
            .flat_map(|(batch, answers)| {
                self.members[batch.members.clone()].iter().zip(answers).map(
                    |(&trait_index, answer)| Verdict {
                        source: batch.source,
                        trait_index,
                        assessment: assessment(&answer),
                    },
                )
            })
            .collect())
    }

    async fn decide(
        &self,
        models: &Models,
        questions: &Questions<'_>,
        batch: usize,
    ) -> Result<Vec<Answer>, ToolFailure> {
        let batch = &self.batches[batch];
        let asked = self.members[batch.members.clone()]
            .iter()
            .map(|&trait_index| questions.question(trait_index))
            .collect::<Vec<_>>();
        let input = input_json(&self.sources[batch.source], self.circumstances);
        provider::decide(models.decision.as_ref(), &input, &asked).await
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

fn serialized_len(value: &Value) -> Result<usize, ToolFailure> {
    serde_json::to_vec(value)
        .map(|bytes| bytes.len())
        .map_err(|error| {
            ToolFailure::platform(format!("failed to measure decision request: {error}"))
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
        let mut plan = Plan::new(None);
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
