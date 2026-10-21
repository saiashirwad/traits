//! End-to-end `check_traits` chunking tests.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use traits::batching::check_traits;
use traits::config::{Config, OperationConfig};
use traits::doc::{Classification, Trait, TraitName};
use traits::error::ToolFailure;
use traits::files::Globs;
use traits::provider::{
    Answer, DecisionModel, DecisionResponse, GenerateRequest, LanguageModel, Models, Question,
    Usage,
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
        _state: &Value,
        questions: &[Question<'_>],
    ) -> Result<DecisionResponse, ToolFailure> {
        let answers = questions
            .iter()
            .map(|question| {
                let name = question.id.to_string();
                let label = if self.violation.contains(&name) {
                    "violation"
                } else if self.inconclusive.contains(&name) {
                    "inconclusive"
                } else {
                    "compliant"
                };
                let choice = question
                    .criteria
                    .iter()
                    .position(|criterion| criterion.label == label)
                    .unwrap();
                let mut probabilities = vec![0.0; question.criteria.len()];
                probabilities[choice] = 1.0;
                Answer {
                    choice,
                    probabilities,
                    confidence: None,
                }
            })
            .collect();
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
        name: TraitName::parse(name).unwrap(),
        subject: "Binding declarations.".to_string(),
        requirement: "Prefer const declarations.".to_string(),
        applicability: None,
        paths: None,
        purpose: "Make immutable bindings explicit.".to_string(),
        guidance: None,
        context: None,
    }
}

fn config_for(root: &std::path::Path) -> Config {
    Config {
        config_root: root.to_path_buf(),
        store: root.join("traits"),
        concurrency: std::num::NonZeroUsize::new(4).unwrap(),
        exclude_gitignored: false,
        excludes: Globs::default(),
        cache_dir: None,
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
        language: Some(Arc::new(NopLanguage)),
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
        Some(&[TraitName::parse("does-not-exist").unwrap()]),
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

#[derive(Default)]
struct Probe {
    fail_on: Option<usize>,
    decision: MockDecision,
    asked: std::sync::Mutex<Vec<(String, String)>>,
    calls: std::sync::atomic::AtomicUsize,
    in_flight: std::sync::atomic::AtomicUsize,
    peak: std::sync::atomic::AtomicUsize,
}

#[async_trait]
impl DecisionModel for Probe {
    async fn decide(
        &self,
        state: &Value,
        questions: &[Question<'_>],
    ) -> Result<DecisionResponse, ToolFailure> {
        use std::sync::atomic::Ordering::SeqCst;
        let file = state["source"]["location"]["path"]
            .as_str()
            .and_then(|path| std::path::Path::new(path).file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.asked.lock().unwrap().extend(
            questions
                .iter()
                .map(|question| (file.clone(), question.id.to_string())),
        );
        if Some(self.calls.fetch_add(1, SeqCst)) == self.fail_on {
            return Err(ToolFailure::ai("request failed"));
        }
        let now = self.in_flight.fetch_add(1, SeqCst) + 1;
        self.peak.fetch_max(now, SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        self.in_flight.fetch_sub(1, SeqCst);
        self.decision.decide(state, questions).await
    }
}

fn probe_models(probe: &Arc<Probe>) -> Models {
    let decision: Arc<dyn DecisionModel> = probe.clone();
    Models {
        decision,
        language: None,
    }
}

#[tokio::test]
async fn concurrency_bounds_every_in_flight_request() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let include: Vec<String> = (0..4)
        .map(|index| {
            let name = format!("f{index}.ts");
            std::fs::write(root.join(&name), "const value = 1").unwrap();
            name
        })
        .collect();
    let store = MockStore {
        traits: (0..48)
            .map(|index| trait_named(&format!("example-trait-{index}")))
            .collect(),
    };
    let probe = Arc::new(Probe::default());
    let mut config = config_for(root);
    config.concurrency = std::num::NonZeroUsize::new(2).unwrap();

    check_traits(&config, &store, &probe_models(&probe), &include, None, None)
        .await
        .unwrap();

    let calls = probe.calls.load(std::sync::atomic::Ordering::SeqCst);
    assert!(calls >= 8, "each file needs several batches, made {calls}");
    assert_eq!(probe.peak.load(std::sync::atomic::Ordering::SeqCst), 2);
}

#[tokio::test]
async fn a_failed_request_stops_the_remaining_batches() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let include: Vec<String> = (0..10)
        .map(|index| {
            let name = format!("f{index}.ts");
            std::fs::write(root.join(&name), "const value = 1").unwrap();
            name
        })
        .collect();
    let store = MockStore {
        traits: vec![trait_named("always-compliant")],
    };
    let probe = Arc::new(Probe {
        fail_on: Some(0),
        ..Probe::default()
    });
    let mut config = config_for(root);
    config.concurrency = std::num::NonZeroUsize::new(2).unwrap();

    let error = check_traits(&config, &store, &probe_models(&probe), &include, None, None)
        .await
        .unwrap_err();

    assert!(matches!(error, ToolFailure::AiError { .. }), "{error:?}");
    assert!(
        probe.calls.load(std::sync::atomic::Ordering::SeqCst) <= 2,
        "no batch starts after the failure"
    );
}

fn asked(probe: &Probe) -> Vec<(String, String)> {
    let mut asked = probe.asked.lock().unwrap().clone();
    asked.sort();
    asked
}

fn pairs(files: &[&str], traits: &[&str]) -> Vec<(String, String)> {
    let mut pairs = files
        .iter()
        .flat_map(|file| {
            traits
                .iter()
                .map(|name| ((*file).to_string(), (*name).to_string()))
        })
        .collect::<Vec<_>>();
    pairs.sort();
    pairs
}

/// A repository of three one-chunk files, a cache in its own tempdir, and two
/// traits, both reported as violations so every pair is a finding.
struct Cached {
    root: tempfile::TempDir,
    cache: tempfile::TempDir,
    store: MockStore,
}

const FILES: [&str; 3] = ["a.ts", "b.ts", "c.ts"];
const TRAITS: [&str; 2] = ["first-trait", "second-trait"];

impl Cached {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        for file in FILES {
            std::fs::write(root.path().join(file), format!("const {} = 1", &file[..1])).unwrap();
        }
        Self {
            root,
            cache: tempfile::tempdir().unwrap(),
            store: MockStore {
                traits: TRAITS.iter().map(|name| trait_named(name)).collect(),
            },
        }
    }

    fn config(&self) -> Config {
        let mut config = config_for(self.root.path());
        config.cache_dir = Some(self.cache.path().to_path_buf());
        config
    }

    fn probe(&self, fail_on: Option<usize>) -> Arc<Probe> {
        Arc::new(Probe {
            fail_on,
            decision: MockDecision {
                violation: TRAITS.iter().map(|name| (*name).to_string()).collect(),
                inconclusive: Vec::new(),
            },
            ..Probe::default()
        })
    }

    async fn check(
        &self,
        config: &Config,
        probe: &Arc<Probe>,
        circumstances: Option<&str>,
    ) -> Result<Vec<traits::doc::TraitEvaluation>, ToolFailure> {
        let include = FILES
            .iter()
            .map(|file| (*file).to_string())
            .collect::<Vec<_>>();
        check_traits(
            config,
            &self.store,
            &probe_models(probe),
            &include,
            None,
            circumstances,
        )
        .await
    }
}

#[tokio::test]
async fn a_second_identical_run_asks_nothing_and_returns_the_same_findings() {
    let repo = Cached::new();
    let cold = repo.probe(None);
    let first = repo.check(&repo.config(), &cold, None).await.unwrap();
    assert_eq!(asked(&cold), pairs(&FILES, &TRAITS));

    let warm = repo.probe(None);
    let second = repo.check(&repo.config(), &warm, None).await.unwrap();
    assert_eq!(warm.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(second, first);
}

#[tokio::test]
async fn editing_one_file_reasks_only_its_chunks_and_keeps_finding_order() {
    let repo = Cached::new();
    repo.check(&repo.config(), &repo.probe(None), None)
        .await
        .unwrap();
    std::fs::write(repo.root.path().join("b.ts"), "let b = 2").unwrap();

    let probe = repo.probe(None);
    let merged = repo.check(&repo.config(), &probe, None).await.unwrap();
    assert_eq!(asked(&probe), pairs(&["b.ts"], &TRAITS));

    let mut uncached = repo.config();
    uncached.cache_dir = None;
    let expected = repo
        .check(&uncached, &repo.probe(None), None)
        .await
        .unwrap();
    assert_eq!(merged, expected);
}

#[tokio::test]
async fn editing_a_trait_reasks_only_that_trait() {
    let mut repo = Cached::new();
    repo.check(&repo.config(), &repo.probe(None), None)
        .await
        .unwrap();
    repo.store.traits[1].requirement = "Prefer let declarations.".to_string();

    let probe = repo.probe(None);
    repo.check(&repo.config(), &probe, None).await.unwrap();
    assert_eq!(asked(&probe), pairs(&FILES, &["second-trait"]));
}

#[tokio::test]
async fn changing_circumstances_reasks_every_pair() {
    let repo = Cached::new();
    repo.check(&repo.config(), &repo.probe(None), None)
        .await
        .unwrap();

    let changed = repo.probe(None);
    repo.check(&repo.config(), &changed, Some("after a refactor"))
        .await
        .unwrap();
    assert_eq!(asked(&changed), pairs(&FILES, &TRAITS));

    let repeated = repo.probe(None);
    repo.check(&repo.config(), &repeated, Some("after a refactor"))
        .await
        .unwrap();
    assert!(asked(&repeated).is_empty());
}

#[tokio::test]
async fn a_disabled_cache_always_asks() {
    let repo = Cached::new();
    let mut config = repo.config();
    config.cache_dir = None;
    for _ in 0..2 {
        let probe = repo.probe(None);
        repo.check(&config, &probe, None).await.unwrap();
        assert_eq!(asked(&probe), pairs(&FILES, &TRAITS));
    }
    assert_eq!(std::fs::read_dir(repo.cache.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn inconclusive_answers_are_asked_again() {
    let repo = Cached::new();
    let inconclusive = || {
        Arc::new(Probe {
            decision: MockDecision {
                violation: Vec::new(),
                inconclusive: vec!["second-trait".to_string()],
            },
            ..Probe::default()
        })
    };
    repo.check(&repo.config(), &inconclusive(), None)
        .await
        .unwrap();

    let probe = inconclusive();
    let findings = repo.check(&repo.config(), &probe, None).await.unwrap();
    assert_eq!(asked(&probe), pairs(&FILES, &["second-trait"]));
    assert_eq!(findings.len(), FILES.len());
}

#[tokio::test]
async fn a_failed_run_keeps_the_batches_that_finished() {
    let repo = Cached::new();
    let mut config = repo.config();
    config.concurrency = std::num::NonZeroUsize::MIN;
    let error = repo
        .check(&config, &repo.probe(Some(2)), None)
        .await
        .unwrap_err();
    assert!(matches!(error, ToolFailure::AiError { .. }), "{error:?}");

    let probe = repo.probe(None);
    repo.check(&config, &probe, None).await.unwrap();
    assert_eq!(asked(&probe), pairs(&["c.ts"], &TRAITS));
}

#[tokio::test]
async fn paths_limit_which_files_each_trait_assesses() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("src/nested")).unwrap();
    for file in ["a.ts", "b.rs", "src/nested/c.rs", "readme.md"] {
        std::fs::write(root.join(file), "const value = 1").unwrap();
    }
    let scoped = |name: &str, paths: &[&str]| Trait {
        paths: Some(
            traits::doc::Paths::parse(paths.iter().map(|glob| (*glob).to_string()).collect())
                .unwrap(),
        ),
        ..trait_named(name)
    };
    let store = MockStore {
        traits: vec![
            scoped("typescript-only", &["*.ts"]),
            scoped("rust-in-src", &["src/**/*.rs"]),
            trait_named("everywhere"),
        ],
    };
    let probe = Arc::new(Probe::default());
    let include = ["a.ts", "b.rs", "src/nested/c.rs", "readme.md"]
        .iter()
        .map(|file| (*file).to_string())
        .collect::<Vec<_>>();
    check_traits(
        &config_for(root),
        &store,
        &probe_models(&probe),
        &include,
        None,
        None,
    )
    .await
    .unwrap();

    let mut expected = vec![
        ("a.ts".to_string(), "everywhere".to_string()),
        ("a.ts".to_string(), "typescript-only".to_string()),
        ("b.rs".to_string(), "everywhere".to_string()),
        ("c.rs".to_string(), "everywhere".to_string()),
        ("c.rs".to_string(), "rust-in-src".to_string()),
        ("readme.md".to_string(), "everywhere".to_string()),
    ];
    expected.sort();
    assert_eq!(asked(&probe), expected);

    let unscoped_files = MockStore {
        traits: vec![scoped("typescript-only", &["*.ts"])],
    };
    let probe = Arc::new(Probe::default());
    check_traits(
        &config_for(root),
        &unscoped_files,
        &probe_models(&probe),
        &["b.rs".to_string(), "readme.md".to_string()],
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(probe.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
}
