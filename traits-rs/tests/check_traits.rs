//! End-to-end `check_traits` chunking tests.

use std::sync::Arc;

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::Value;

use traits::batching::check_traits;
use traits::config::{Config, OperationConfig};
use traits::doc::{Classification, Trait, TraitName};
use traits::error::ToolFailure;
use traits::provider::{
    ClassifyAnswer, ClassifyQuestion, DecisionModel, DecisionResponse, GenerateRequest,
    LanguageModel, Models, Usage,
};
use traits::store::TraitStore;

struct MockStore {
    traits: Vec<Trait>,
}

#[async_trait]
impl TraitStore for MockStore {
    async fn ls(&self) -> Result<Vec<Trait>, ToolFailure> {
        Ok(self.traits.clone())
    }

    async fn get(&self, name: &TraitName) -> Result<Trait, ToolFailure> {
        self.traits
            .iter()
            .find(|trait_value| &trait_value.name == name)
            .cloned()
            .ok_or_else(|| ToolFailure::not_found(name.as_str()))
    }

    async fn add(&self, _trait_value: Trait) -> Result<Trait, ToolFailure> {
        Err(ToolFailure::platform("not used"))
    }

    async fn update(&self, _trait_value: Trait) -> Result<(), ToolFailure> {
        Err(ToolFailure::platform("not used"))
    }

    async fn rm(&self, _names: &[TraitName]) -> Result<(), ToolFailure> {
        Err(ToolFailure::platform("not used"))
    }
}

#[derive(Default)]
struct MockDecision {
    violation: Vec<String>,
    inconclusive: Vec<String>,
}

#[async_trait]
impl DecisionModel for MockDecision {
    async fn decide(
        &self,
        _state: Value,
        questions: Vec<(String, ClassifyQuestion)>,
    ) -> Result<DecisionResponse, ToolFailure> {
        let mut answers = IndexMap::new();
        for (name, _) in questions {
            let label = if self.violation.contains(&name) {
                "violation"
            } else if self.inconclusive.contains(&name) {
                "inconclusive"
            } else {
                "compliant"
            };
            let mut probabilities = IndexMap::new();
            for candidate in ["compliant", "violation", "not-applicable", "inconclusive"] {
                probabilities.insert(
                    candidate.to_string(),
                    if candidate == label { 1.0 } else { 0.0 },
                );
            }
            answers.insert(
                name,
                ClassifyAnswer {
                    label: label.to_string(),
                    probabilities,
                    confidence: None,
                },
            );
        }
        Ok(DecisionResponse {
            answers,
            usage: Usage::default(),
        })
    }
}

struct NopLanguage;

#[async_trait]
impl LanguageModel for NopLanguage {
    async fn generate_object(&self, _request: GenerateRequest) -> Result<Value, ToolFailure> {
        Err(ToolFailure::platform(
            "language model is not available in tests",
        ))
    }
}

fn trait_named(name: &str) -> Trait {
    Trait {
        name: TraitName(name.to_string()),
        subject: "Binding declarations.".to_string(),
        requirement: "Prefer const declarations.".to_string(),
        applicability: None,
        purpose: "Make immutable bindings explicit.".to_string(),
        guidance: None,
        context: None,
    }
}

fn config_for(root: &std::path::Path) -> Config {
    Config {
        config_root: root.to_path_buf(),
        store: root.join("traits"),
        concurrency: 4,
        exclude_gitignored: false,
        excludes: Vec::new(),
        default_provider: None,
        classify: OperationConfig::default(),
        generate: OperationConfig::default(),
        providers: Vec::new(),
    }
}

fn lines(count: usize) -> String {
    (1..=count)
        .map(|index| format!("line {index}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn models(violation: &[&str]) -> Models {
    let decision: Arc<dyn DecisionModel> = Arc::new(MockDecision {
        violation: violation.iter().map(|name| (*name).to_string()).collect(),
        inconclusive: Vec::new(),
    });
    Models {
        decision,
        language: Arc::new(NopLanguage),
    }
}

fn start_lines(findings: &[traits::doc::TraitEvaluation]) -> Vec<(String, u32)> {
    findings
        .iter()
        .map(|finding| {
            let path = match &finding.source.location {
                Some(traits::doc::SourceLocation::File { path, .. }) => path.clone(),
                other => panic!("expected file location, got {other:?}"),
            };
            let start = match &finding.source.location {
                Some(traits::doc::SourceLocation::File { start_line, .. }) => *start_line,
                _ => unreachable!(),
            };
            (path, start)
        })
        .collect()
}

#[tokio::test]
async fn chunks_files_at_two_hundred_lines_and_keeps_review_findings() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("f199.ts"), lines(199)).unwrap();
    std::fs::write(root.join("f200.ts"), lines(200)).unwrap();
    std::fs::write(root.join("f201.ts"), lines(201)).unwrap();
    std::fs::write(root.join("ftrail.ts"), format!("{}\n", lines(200))).unwrap();

    let store = MockStore {
        traits: vec![
            trait_named("always-violation"),
            trait_named("always-compliant"),
        ],
    };
    let models = models(&["always-violation"]);
    let include: Vec<String> = ["f199.ts", "f200.ts", "f201.ts", "ftrail.ts"]
        .iter()
        .map(|name| (*name).to_string())
        .collect();

    let findings = check_traits(&config_for(root), &store, &models, &include, None, None)
        .await
        .unwrap();

    assert_eq!(findings.len(), 6, "one violation finding per chunk");
    assert!(
        findings
            .iter()
            .all(|finding| finding.assessment.classification == Classification::Violation),
        "compliant results are filtered out"
    );
    let expected: Vec<(String, u32)> = [
        ("f199.ts", 1),
        ("f200.ts", 1),
        ("f201.ts", 1),
        ("f201.ts", 201),
        ("ftrail.ts", 1),
        ("ftrail.ts", 201),
    ]
    .iter()
    .map(|(name, start)| (root.join(name).to_string_lossy().into_owned(), *start))
    .collect();
    assert_eq!(start_lines(&findings), expected);
}

#[tokio::test]
async fn empty_file_yields_a_single_chunk_and_ignore_directives_skip_traits() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("empty.ts"), "").unwrap();
    std::fs::write(
        root.join("ignored.ts"),
        "// traits-ignore-file always-violation: legacy module\nconst value = 1",
    )
    .unwrap();

    let store = MockStore {
        traits: vec![
            trait_named("always-violation"),
            trait_named("always-compliant"),
        ],
    };
    let models = models(&["always-violation"]);

    let empty = check_traits(
        &config_for(root),
        &store,
        &models,
        &["empty.ts".to_string()],
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        start_lines(&empty),
        vec![(root.join("empty.ts").to_string_lossy().into_owned(), 1)]
    );

    let ignored = check_traits(
        &config_for(root),
        &store,
        &models,
        &["ignored.ts".to_string()],
        None,
        None,
    )
    .await
    .unwrap();
    assert!(
        ignored.is_empty(),
        "the violating trait is ignored for the whole file"
    );
}

#[tokio::test]
async fn unknown_trait_names_fail_and_empty_selections_return_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("f.ts"), "const value = 1").unwrap();
    let store = MockStore {
        traits: vec![trait_named("always-violation")],
    };
    let models = models(&["always-violation"]);

    let missing = check_traits(
        &config_for(root),
        &store,
        &models,
        &["f.ts".to_string()],
        Some(&[TraitName("does-not-exist".to_string())]),
        None,
    )
    .await
    .unwrap_err();
    assert!(
        matches!(missing, ToolFailure::TraitNotFoundError { .. }),
        "got {missing:?}"
    );

    let none = check_traits(&config_for(root), &store, &models, &[], None, None)
        .await
        .unwrap();
    assert!(none.is_empty());

    let selected_none: Vec<TraitName> = Vec::new();
    let focused = check_traits(
        &config_for(root),
        &store,
        &models,
        &["f.ts".to_string()],
        Some(&selected_none),
        None,
    )
    .await
    .unwrap();
    assert!(focused.is_empty());
}
