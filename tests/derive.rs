//! Port of `traits/test/TraitDerivation.test.ts`.
//!
//! Decision and language models are scripted per question key, the store is an
//! in-memory double that records mutation order, and approval is scripted.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::{Value, json};

use traits::conflict::{FailReason, TraitConflict};
use traits::derive::{Approval, Approver, DeriveParams, TraitDerivation, derive};
use traits::doc::{Trait, TraitName};
use traits::error::ToolFailure;
use traits::provider::{
    Answer, DecisionModel, DecisionResponse, GenerateRequest, LanguageModel, Models, Question,
    Usage,
};
use traits::store::TraitStore;

fn trait_name(value: &str) -> TraitName {
    TraitName::parse(value).expect("valid trait name")
}

fn original() -> Trait {
    Trait {
        name: trait_name("indentation"),
        subject: "Code indentation.".to_string(),
        requirement: "Use tabs for indentation.".to_string(),
        applicability: None,
        purpose: "Keep indentation consistent.".to_string(),
        guidance: None,
        context: None,
    }
}

fn two_spaces() -> Trait {
    Trait {
        requirement: "Use two spaces for indentation.".to_string(),
        ..original()
    }
}

fn full_proposal(name: &str) -> Trait {
    Trait {
        name: trait_name(name),
        subject: "Code indentation.".to_string(),
        requirement: "Use two spaces for indentation.".to_string(),
        applicability: Some("Editing state in TSX, except generated components.".to_string()),
        purpose: "Keep state updates consistent.".to_string(),
        guidance: Some("Update the state operation.".to_string()),
        context: Some("A supporting example.\n".to_string()),
    }
}

fn unrelated() -> Trait {
    Trait {
        name: trait_name("unrelated"),
        subject: "Other artifacts.".to_string(),
        requirement: "Keep unrelated requirements.".to_string(),
        applicability: None,
        purpose: "Preserve independent conventions.".to_string(),
        guidance: None,
        context: None,
    }
}

fn other() -> Trait {
    Trait {
        name: trait_name("other"),
        subject: "Other artifacts.".to_string(),
        requirement: "Keep other requirements.".to_string(),
        applicability: None,
        purpose: "Preserve other conventions.".to_string(),
        guidance: None,
        context: None,
    }
}

struct MockDecision {
    relationships: HashMap<String, String>,
    coverage: HashMap<String, String>,
}

#[async_trait]
impl DecisionModel for MockDecision {
    async fn decide(
        &self,
        state: &Value,
        questions: &[Question<'_>],
    ) -> Result<DecisionResponse, ToolFailure> {
        let question = questions[0];
        let key = question.id;
        let name = state
            .get("trait")
            .and_then(|trait_value| trait_value.get("name"))
            .and_then(|name| name.as_str());
        let label: String = match name {
            Some(name) => {
                if key == "coverage" {
                    self.coverage
                        .get(name)
                        .cloned()
                        .unwrap_or_else(|| "complete".to_string())
                } else {
                    self.relationships
                        .get(name)
                        .cloned()
                        .unwrap_or_else(|| panic!("missing relationship for {name}"))
                }
            }
            None => "atomic".to_string(),
        };
        let choice = question
            .criteria
            .iter()
            .position(|criterion| criterion.label == label)
            .unwrap_or_else(|| panic!("{label} is not a {key} label"));
        let mut probabilities = vec![0.0; question.criteria.len()];
        probabilities[choice] = 1.0;
        Ok(DecisionResponse {
            answers: vec![Answer {
                choice,
                probabilities,
                confidence: None,
            }],
            usage: Usage::default(),
        })
    }
}

struct MockLanguage {
    response: Value,
    requests: Mutex<Vec<GenerateRequest>>,
}

#[async_trait]
impl LanguageModel for MockLanguage {
    async fn generate_object(&self, request: GenerateRequest) -> Result<Value, ToolFailure> {
        self.requests.lock().unwrap().push(request);
        Ok(self.response.clone())
    }
}

struct MockStore {
    stored: Mutex<IndexMap<TraitName, Trait>>,
    mutations: Mutex<Vec<String>>,
}

impl MockStore {
    fn new(traits: Vec<Trait>) -> Self {
        let stored = traits
            .into_iter()
            .map(|trait_value| (trait_value.name.clone(), trait_value))
            .collect::<IndexMap<_, _>>();
        Self {
            stored: Mutex::new(stored),
            mutations: Mutex::new(Vec::new()),
        }
    }

    fn mutations(&self) -> Vec<String> {
        self.mutations.lock().unwrap().clone()
    }

    fn values(&self) -> Vec<Trait> {
        self.stored.lock().unwrap().values().cloned().collect()
    }

    fn get(&self, name: &TraitName) -> Option<Trait> {
        self.stored.lock().unwrap().get(name).cloned()
    }

    fn keys(&self) -> Vec<TraitName> {
        self.stored.lock().unwrap().keys().cloned().collect()
    }
}

#[async_trait]
impl TraitStore for MockStore {
    async fn ls(&self) -> Result<Vec<Trait>, ToolFailure> {
        Ok(self.values())
    }

    async fn get(&self, name: &TraitName) -> Result<Trait, ToolFailure> {
        self.get(name)
            .ok_or_else(|| ToolFailure::not_found(name.as_str()))
    }

    async fn add(&self, trait_value: Trait) -> Result<Trait, ToolFailure> {
        if self.stored.lock().unwrap().contains_key(&trait_value.name) {
            return Err(ToolFailure::already_exists(trait_value.name.as_str()));
        }
        self.mutations.lock().unwrap().push("add".to_string());
        self.stored
            .lock()
            .unwrap()
            .insert(trait_value.name.clone(), trait_value.clone());
        Ok(trait_value)
    }

    async fn update(&self, trait_value: Trait) -> Result<(), ToolFailure> {
        self.mutations.lock().unwrap().push("update".to_string());
        self.stored
            .lock()
            .unwrap()
            .insert(trait_value.name.clone(), trait_value);
        Ok(())
    }

    async fn rm(&self, names: &[TraitName]) -> Result<(), ToolFailure> {
        self.mutations.lock().unwrap().push("rm".to_string());
        let mut stored = self.stored.lock().unwrap();
        for name in names {
            stored.shift_remove(name);
        }
        Ok(())
    }
}

struct TestApprover {
    forced: Option<Approval>,
}

#[async_trait]
impl Approver for TestApprover {
    async fn request(
        &self,
        message: String,
        approved: Option<bool>,
    ) -> Result<Approval, ToolFailure> {
        if let Some(approval) = &self.forced {
            return Ok(approval.clone());
        }
        Ok(if approved == Some(true) {
            Approval::Approved {}
        } else {
            Approval::ApprovalRequired { message }
        })
    }
}

struct Harness {
    models: Models,
    store: MockStore,
    language: Arc<MockLanguage>,
    approver: TestApprover,
}

fn harness(
    stored: Vec<Trait>,
    relationships: &[(&str, &str)],
    coverage: &[(&str, &str)],
    response: Value,
    forced: Option<Approval>,
) -> Harness {
    let language = Arc::new(MockLanguage {
        response,
        requests: Mutex::new(Vec::new()),
    });
    let decision = MockDecision {
        relationships: relationships
            .iter()
            .map(|(name, label)| (name.to_string(), label.to_string()))
            .collect(),
        coverage: coverage
            .iter()
            .map(|(name, label)| (name.to_string(), label.to_string()))
            .collect(),
    };
    let decision: Arc<dyn DecisionModel> = Arc::new(decision);
    let language_model: Arc<dyn LanguageModel> = language.clone();
    Harness {
        models: Models {
            decision,
            language: Some(language_model),
        },
        store: MockStore::new(stored),
        language,
        approver: TestApprover { forced },
    }
}

fn derive_params(proposed_trait: Trait, approved: Option<bool>) -> DeriveParams {
    DeriveParams {
        proposed_trait,
        approved,
    }
}

#[tokio::test]
async fn approves_and_persists_without_generation_for_creation_and_clarification() {
    let proposal = full_proposal("state-updates");
    for (relationship, stored_name) in [
        ("distinct", "state-updates"),
        ("clarification", "indentation"),
    ] {
        let h = harness(
            vec![original()],
            &[("indentation", relationship)],
            &[],
            Value::Null,
            None,
        );

        let pending = derive(
            &h.models,
            &h.store,
            &h.approver,
            derive_params(proposal.clone(), None),
        )
        .await
        .unwrap();
        match &pending {
            TraitDerivation::ApprovalRequired { message } => {
                if relationship == "distinct" {
                    assert!(message.contains("Save trait state-updates?"), "{message}");
                    assert!(
                        message.contains("Editing state in TSX, except generated components."),
                        "{message}"
                    );
                } else {
                    assert!(message.contains("Clarify trait indentation?"), "{message}");
                    assert!(message.contains("Current trait:"), "{message}");
                    assert!(message.contains("Replacement trait:"), "{message}");
                }
            }
            other => panic!("expected ApprovalRequired, got {other:?}"),
        }
        assert!(h.store.mutations().is_empty());

        let result = derive(
            &h.models,
            &h.store,
            &h.approver,
            derive_params(proposal.clone(), Some(true)),
        )
        .await
        .unwrap();
        let mut expected_trait = proposal.clone();
        expected_trait.name = trait_name(stored_name);
        match result {
            TraitDerivation::Created {
                r#trait,
                instructions,
            } => {
                assert_eq!(relationship, "distinct");
                assert_eq!(r#trait, expected_trait);
                assert!(instructions.contains("Saved traits: state-updates."));
            }
            TraitDerivation::Updated {
                r#trait,
                instructions,
            } => {
                assert_eq!(relationship, "clarification");
                assert_eq!(r#trait, expected_trait);
                assert!(instructions.contains("Saved traits: indentation."));
            }
            other => panic!("expected a saved outcome, got {other:?}"),
        }
        assert_eq!(h.store.get(&trait_name(stored_name)), Some(expected_trait));

        if relationship == "clarification" {
            let cleared = derive(
                &h.models,
                &h.store,
                &h.approver,
                derive_params(two_spaces(), Some(true)),
            )
            .await
            .unwrap();
            match cleared {
                TraitDerivation::Updated { r#trait, .. } => assert_eq!(r#trait, two_spaces()),
                other => panic!("expected Updated, got {other:?}"),
            }
            assert_eq!(h.store.get(&trait_name("indentation")), Some(two_spaces()));
        }
        assert!(h.language.requests.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn review_instructions_require_whole_project_checks() {
    let proposal = full_proposal("state-updates");
    for relationship in ["distinct", "clarification", "conflict"] {
        let response = if relationship == "conflict" {
            json!({ "traits": [two_spaces()] })
        } else {
            Value::Null
        };
        let h = harness(
            vec![original()],
            &[("indentation", relationship)],
            &[],
            response,
            None,
        );
        let result = derive(
            &h.models,
            &h.store,
            &h.approver,
            derive_params(proposal.clone(), Some(true)),
        )
        .await
        .unwrap();
        let instructions = match &result {
            TraitDerivation::Created { instructions, .. }
            | TraitDerivation::Updated { instructions, .. }
            | TraitDerivation::Resolved { instructions, .. } => instructions.clone(),
            other => panic!("expected a saved outcome, got {other:?}"),
        };
        let saved = if relationship == "distinct" {
            "state-updates"
        } else {
            "indentation"
        };
        for requirement in [
            format!("Saved traits: {saved}."),
            "Required post-save trait checks: This task is incomplete until you run check_traits against all eligible project-owned files".to_string(),
            "traitNames set to the saved trait names".to_string(),
            "Checking only changed files or the file that prompted the trait does not satisfy this requirement".to_string(),
            "project-owned source, authored configuration, and documentation".to_string(),
            "Respect trait applicability and configured exclusions".to_string(),
            "dependencies, reference repositories, generated output, lockfiles, Git metadata, secrets, .env files, credentials, and binary assets".to_string(),
            "minimal behavior-preserving edits".to_string(),
            "rerun check_traits on corrected files, and run relevant verification".to_string(),
            "ask before behavior-changing fixes or ambiguous interpretations".to_string(),
            "explicitly report the blocker and state that the required post-save trait checks remain incomplete".to_string(),
            "Saving a trait does not itself check or update project files".to_string(),
        ] {
            assert!(instructions.contains(&requirement), "missing: {requirement}");
        }
    }
}

#[tokio::test]
async fn conflict_requires_approval_before_generation() {
    let h = harness(
        vec![original()],
        &[("indentation", "conflict")],
        &[],
        Value::Null,
        None,
    );
    let result = derive(
        &h.models,
        &h.store,
        &h.approver,
        derive_params(two_spaces(), None),
    )
    .await
    .unwrap();
    assert!(matches!(result, TraitDerivation::ApprovalRequired { .. }));
    assert!(h.language.requests.lock().unwrap().is_empty());
    assert!(h.store.mutations().is_empty());
}

#[tokio::test]
async fn reconcile_reuses_selected_names_and_deletes_unused_originals() {
    let mut stored_original = original();
    stored_original.applicability = Some("TypeScript, except generated files.".to_string());
    stored_original.context = Some("Original context.".to_string());
    let h = harness(
        vec![stored_original],
        &[("indentation", "conflict")],
        &[],
        json!({ "traits": [two_spaces()] }),
        None,
    );
    let result = derive(
        &h.models,
        &h.store,
        &h.approver,
        derive_params(two_spaces(), Some(true)),
    )
    .await
    .unwrap();
    match &result {
        TraitDerivation::Resolved { instructions, .. } => assert!(instructions
            .contains("This task is incomplete until you run check_traits against all eligible project-owned files")),
        other => panic!("expected Resolved, got {other:?}"),
    }
    assert_eq!(h.store.mutations(), vec!["update", "rm"]);
    assert_eq!(h.store.get(&trait_name("indentation")), Some(two_spaces()));
    assert_eq!(
        h.store
            .values()
            .iter()
            .map(|trait_value| trait_value.requirement.clone())
            .collect::<Vec<_>>(),
        vec![two_spaces().requirement]
    );
    let requests = h.language.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(
        requests[0]
            .system
            .contains("Return only a JSON object with this shape: {\"traits\":[{\"name\":")
    );
}

#[tokio::test]
async fn invalid_generation_leaves_the_store_untouched() {
    for response in [
        Value::String("Use two spaces for indentation.".to_string()),
        json!({ "traits": [] }),
        json!({ "traits": [{
            "name": "../escape",
            "subject": "Code indentation.",
            "requirement": "Use two spaces.",
            "purpose": "Consistency."
        }] }),
        json!({ "traits": [{
            "name": "indentation",
            "subject": "Code indentation.",
            "requirement": "Use two spaces.",
            "purpose": "Consistency.",
            "guidance": "  "
        }] }),
    ] {
        let h = harness(
            vec![original()],
            &[("indentation", "conflict")],
            &[],
            response,
            None,
        );
        let error = derive(
            &h.models,
            &h.store,
            &h.approver,
            derive_params(two_spaces(), Some(true)),
        )
        .await
        .unwrap_err();
        assert!(matches!(error, ToolFailure::AiError { .. }), "{error:?}");
        assert!(h.store.mutations().is_empty());
        assert_eq!(h.store.values(), vec![original()]);
    }
}

#[tokio::test]
async fn rejects_duplicate_and_reserved_names_before_writes() {
    for names in [
        vec!["replacement", "replacement"],
        vec!["replacement", "unrelated"],
    ] {
        let generated = names
            .iter()
            .map(|name| {
                let mut draft = two_spaces();
                draft.name = trait_name(name);
                draft
            })
            .collect::<Vec<_>>();
        let h = harness(
            vec![original(), unrelated()],
            &[("indentation", "conflict"), ("unrelated", "distinct")],
            &[],
            json!({ "traits": generated }),
            None,
        );
        let error = derive(
            &h.models,
            &h.store,
            &h.approver,
            derive_params(two_spaces(), Some(true)),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(error, ToolFailure::TraitAlreadyExistsError { .. }),
            "{error:?}"
        );
        assert!(h.store.mutations().is_empty());
        assert_eq!(h.store.values(), vec![original(), unrelated()]);
    }
}

#[tokio::test]
async fn saves_a_new_replacement_name_before_removing_the_original() {
    let replacement_name = trait_name("two-space-indentation");
    let mut replacement = two_spaces();
    replacement.name = replacement_name.clone();
    let h = harness(
        vec![original()],
        &[("indentation", "conflict")],
        &[],
        json!({ "traits": [replacement] }),
        None,
    );
    let result = derive(
        &h.models,
        &h.store,
        &h.approver,
        derive_params(two_spaces(), Some(true)),
    )
    .await
    .unwrap();
    assert!(matches!(result, TraitDerivation::Resolved { .. }));
    assert_eq!(h.store.mutations(), vec!["add", "rm"]);
    assert_eq!(h.store.keys(), vec![replacement_name]);
}

#[tokio::test]
async fn covered_proposal_is_reported_without_writes() {
    let h = harness(
        vec![original()],
        &[("indentation", "covered")],
        &[("indentation", "complete")],
        Value::Null,
        None,
    );
    let result = derive(
        &h.models,
        &h.store,
        &h.approver,
        derive_params(two_spaces(), Some(true)),
    )
    .await
    .unwrap();
    match result {
        TraitDerivation::Covered { traits } => assert_eq!(traits, vec![original()]),
        other => panic!("expected Covered, got {other:?}"),
    }
    assert!(h.store.mutations().is_empty());
    assert!(h.language.requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn coverage_gap_reconciles() {
    let h = harness(
        vec![original()],
        &[("indentation", "covered")],
        &[("indentation", "incomplete")],
        json!({ "traits": [two_spaces()] }),
        None,
    );
    let result = derive(
        &h.models,
        &h.store,
        &h.approver,
        derive_params(two_spaces(), Some(true)),
    )
    .await
    .unwrap();
    assert!(matches!(result, TraitDerivation::Resolved { .. }));
    assert_eq!(h.store.mutations(), vec!["update", "rm"]);
}

#[tokio::test]
async fn multiple_clarifications_are_reported_without_writes() {
    let h = harness(
        vec![original(), other()],
        &[("indentation", "clarification"), ("other", "clarification")],
        &[],
        Value::Null,
        None,
    );
    let result = derive(
        &h.models,
        &h.store,
        &h.approver,
        derive_params(two_spaces(), Some(true)),
    )
    .await
    .unwrap();
    match result {
        TraitDerivation::Clarification {
            traits,
            proposed_trait,
        } => {
            assert_eq!(traits, vec![original(), other()]);
            assert!(
                proposed_trait.contains("name: indentation"),
                "{proposed_trait}"
            );
        }
        other => panic!("expected Clarification, got {other:?}"),
    }
    assert!(h.store.mutations().is_empty());
}

#[tokio::test]
async fn inconclusive_findings_block_all_writes() {
    let h = harness(
        vec![original(), other()],
        &[("indentation", "inconclusive"), ("other", "conflict")],
        &[],
        Value::Null,
        None,
    );
    let result = derive(
        &h.models,
        &h.store,
        &h.approver,
        derive_params(two_spaces(), Some(true)),
    )
    .await
    .unwrap();
    match result {
        TraitDerivation::Inconclusive { findings } => assert_eq!(findings.len(), 2),
        other => panic!("expected Inconclusive, got {other:?}"),
    }
    assert!(h.store.mutations().is_empty());
    assert!(h.language.requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn approver_outcomes_pass_through_even_when_approved() {
    let h = harness(
        vec![original()],
        &[("indentation", "distinct")],
        &[],
        Value::Null,
        Some(Approval::Declined {}),
    );
    let result = derive(
        &h.models,
        &h.store,
        &h.approver,
        derive_params(full_proposal("state-updates"), Some(true)),
    )
    .await
    .unwrap();
    assert!(matches!(result, TraitDerivation::Declined {}));
    assert!(h.store.mutations().is_empty());

    let h = harness(
        vec![original()],
        &[("indentation", "clarification")],
        &[],
        Value::Null,
        Some(Approval::RevisionRequired {
            feedback: "reword it".to_string(),
        }),
    );
    let result = derive(
        &h.models,
        &h.store,
        &h.approver,
        derive_params(full_proposal("state-updates"), Some(true)),
    )
    .await
    .unwrap();
    match result {
        TraitDerivation::RevisionRequired { feedback } => assert_eq!(feedback, "reword it"),
        other => panic!("expected RevisionRequired, got {other:?}"),
    }
    assert!(h.store.mutations().is_empty());

    let h = harness(
        vec![original()],
        &[("indentation", "conflict")],
        &[],
        Value::Null,
        Some(Approval::Declined {}),
    );
    let result = derive(
        &h.models,
        &h.store,
        &h.approver,
        derive_params(two_spaces(), Some(true)),
    )
    .await
    .unwrap();
    assert!(matches!(result, TraitDerivation::Declined {}));
    assert!(h.language.requests.lock().unwrap().is_empty());
    assert!(h.store.mutations().is_empty());
}

#[tokio::test]
async fn tagged_serialization_matches_the_wire_contract() {
    let conflict = TraitConflict::Clarification {
        r#trait: original(),
        proposed_trait: "proposed".to_string(),
    };
    let value = serde_json::to_value(&conflict).unwrap();
    assert_eq!(value["_tag"], "Clarification");
    assert_eq!(value["trait"]["name"], "indentation");
    assert_eq!(value["proposedTrait"], "proposed");

    let fail = TraitConflict::Fail {
        existing: original(),
        proposed: "p".to_string(),
        reason: FailReason::CoverageGap,
    };
    assert_eq!(
        serde_json::to_value(&fail).unwrap()["reason"],
        "coverage_gap"
    );

    let created = TraitDerivation::Created {
        r#trait: original(),
        instructions: "i".to_string(),
    };
    let value = serde_json::to_value(&created).unwrap();
    assert_eq!(value["_tag"], "Created");
    assert_eq!(value["trait"]["name"], "indentation");
    assert!(value.get("r#trait").is_none());
}

#[tokio::test]
async fn existing_proposal_name_is_rejected() {
    let h = harness(
        vec![original()],
        &[("indentation", "distinct")],
        &[],
        Value::Null,
        None,
    );
    let error = derive(
        &h.models,
        &h.store,
        &h.approver,
        derive_params(two_spaces(), Some(true)),
    )
    .await
    .unwrap_err();
    assert!(
        matches!(error, ToolFailure::TraitAlreadyExistsError { .. }),
        "{error:?}"
    );
    assert!(h.store.mutations().is_empty());
}
