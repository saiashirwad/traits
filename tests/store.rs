use std::path::Path;

use tempfile::TempDir;
use traits::doc::{Trait, TraitName, serialize};
use traits::error::ToolFailure;
use traits::store::{FileSystemStore, TraitStore};

fn name(value: &str) -> TraitName {
    TraitName::parse(value).expect("valid trait name")
}

fn trait_with(name_value: &str) -> Trait {
    Trait {
        name: name(name_value),
        subject: "Asynchronous tests.".to_string(),
        requirement: "Original".to_string(),
        applicability: Some("In TypeScript and TSX, except when a synchronous framework contract must be preserved.".to_string()),
        paths: None,
        purpose: "Preserve test lifecycle behavior.".to_string(),
        guidance: Some("Use the Effect-aware test APIs.".to_string()),
        context: Some("Supporting context.\n\n---\nExample.\n".to_string()),
    }
}

async fn open(dir: &TempDir) -> FileSystemStore {
    FileSystemStore::open(dir.path().to_path_buf())
        .await
        .expect("opens store")
}

fn file(dir: &TempDir, name_value: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name_value}.md"))
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).expect("reads file")
}

const RECOVERY_SUBSTRINGS: [&str; 10] = [
    "Ask the developer:",
    "Do not change files before approval",
    "If declined, leave files unchanged",
    "inspect the entire affected trait store",
    "authoritative requirements, applicability, exceptions, examples",
    "ask a focused question",
    "Validate all files",
    "original arguments",
    "A server restart is not necessary",
    "original operation failed",
];

#[tokio::test]
async fn open_creates_the_root_recursively() {
    let dir = TempDir::new().expect("tempdir");
    let nested = dir.path().join("a").join("b");
    let store = FileSystemStore::open(nested.clone()).await.expect("opens");
    assert_eq!(store.root(), nested.as_path());
    assert!(nested.is_dir());
}

#[tokio::test]
async fn lifecycle_add_get_update_rm() {
    let dir = TempDir::new().expect("tempdir");
    let store = open(&dir).await;
    let initial = trait_with("test-trait");

    let added = store.add(initial.clone()).await.expect("adds");
    assert_eq!(added, initial);
    assert_eq!(store.get(&initial.name).await.expect("gets"), initial);

    let updated = Trait {
        requirement: "Replacement".to_string(),
        context: Some("Updated context.\n".to_string()),
        ..initial.clone()
    };
    store.update(updated.clone()).await.expect("updates");
    assert_eq!(store.get(&initial.name).await.expect("gets"), updated);
    assert_eq!(store.ls().await.expect("lists"), vec![updated.clone()]);
    let entries = std::fs::read_dir(store.root())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(entries, ["test-trait.md"], "no staged file is left behind");

    let text = read(&file(&dir, "test-trait"));
    assert!(text.contains("name: test-trait"), "{text}");
    assert!(!text.contains("id:"), "{text}");

    let entries: Vec<String> = std::fs::read_dir(dir.path())
        .expect("read dir")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(entries, vec!["test-trait.md".to_string()]);

    let collision = store
        .add(initial.clone())
        .await
        .expect_err("duplicate add fails");
    assert!(
        matches!(&collision, ToolFailure::TraitAlreadyExistsError { name, .. } if name == "test-trait")
    );
    assert_eq!(store.get(&initial.name).await.expect("gets"), updated);

    let renamed = Trait {
        name: name("renamed-trait"),
        ..updated.clone()
    };
    let missing_update = store
        .update(renamed.clone())
        .await
        .expect_err("update of missing trait fails");
    assert!(matches!(
        missing_update,
        ToolFailure::TraitNotFoundError { .. }
    ));

    store.add(renamed.clone()).await.expect("adds renamed");
    store
        .rm(std::slice::from_ref(&initial.name))
        .await
        .expect("removes initial");
    assert_eq!(store.ls().await.expect("lists"), vec![renamed.clone()]);

    let missing = store
        .get(&initial.name)
        .await
        .expect_err("get of removed trait fails");
    assert!(
        matches!(&missing, ToolFailure::TraitNotFoundError { name, .. } if name == "test-trait")
    );

    store
        .rm(std::slice::from_ref(&renamed.name))
        .await
        .expect("removes renamed");
    assert_eq!(store.ls().await.expect("lists"), Vec::<Trait>::new());
}

#[cfg(unix)]
#[tokio::test]
async fn update_writes_through_symlinks_and_keeps_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().expect("tempdir");
    let store = open(&dir).await;
    let initial = trait_with("test-trait");
    let elsewhere = TempDir::new().expect("tempdir");
    let real = elsewhere.path().join("real.md");
    std::fs::write(&real, traits::doc::serialize(&initial)).unwrap();
    std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o640)).unwrap();
    std::os::unix::fs::symlink(&real, file(&dir, "test-trait")).unwrap();

    let updated = Trait {
        requirement: "Replacement".to_string(),
        ..initial
    };
    store.update(updated.clone()).await.expect("updates");

    assert!(file(&dir, "test-trait").is_symlink());
    assert_eq!(read(&real), traits::doc::serialize(&updated));
    assert_eq!(
        std::fs::metadata(&real).unwrap().permissions().mode() & 0o777,
        0o640
    );
}

#[tokio::test]
async fn get_missing_trait_reports_not_found() {
    let dir = TempDir::new().expect("tempdir");
    let store = open(&dir).await;
    let failure = store.get(&name("missing-trait")).await.expect_err("fails");
    assert!(
        matches!(&failure, ToolFailure::TraitNotFoundError { name, .. } if name == "missing-trait")
    );
    assert_eq!(failure.message(), "Trait missing-trait was not found");
}

#[tokio::test]
async fn filename_mismatch_is_rejected_without_mutation() {
    let dir = TempDir::new().expect("tempdir");
    let store = open(&dir).await;
    let actual = Trait {
        subject: "An artifact.".to_string(),
        requirement: "Preserve its identity.".to_string(),
        applicability: None,
        purpose: "Avoid identity mismatches.".to_string(),
        guidance: None,
        context: None,
        ..trait_with("actual-name")
    };
    let wrong = name("wrong-name");
    let path = file(&dir, "wrong-name");
    std::fs::write(&path, serialize(&actual)).expect("writes fixture");
    let original = read(&path);

    let get_failure = store.get(&wrong).await.expect_err("get fails");
    assert_mismatch(&get_failure, &path, "actual-name.md");

    let update_failure = store
        .update(Trait {
            name: wrong.clone(),
            ..actual.clone()
        })
        .await
        .expect_err("update fails");
    assert_mismatch(&update_failure, &path, "actual-name.md");

    let rm_failure = store
        .rm(std::slice::from_ref(&wrong))
        .await
        .expect_err("rm fails");
    assert_mismatch(&rm_failure, &path, "actual-name.md");

    let ls_failure = store.ls().await.expect_err("ls fails");
    assert_mismatch(&ls_failure, &path, "actual-name.md");

    assert_eq!(read(&path), original, "file must be untouched");
    assert!(path.exists(), "file must still exist");

    // The frontmatter name is authoritative once the filename matches.
    store.add(actual.clone()).await.expect("adds");
    assert_eq!(store.get(&actual.name).await.expect("gets"), actual);
}

fn assert_mismatch(failure: &ToolFailure, path: &Path, expected: &str) {
    match failure {
        ToolFailure::TraitFilenameMismatchError {
            path: got,
            expected_filename,
            ..
        } => {
            assert_eq!(got.as_str(), path.to_string_lossy().as_ref());
            assert_eq!(expected_filename.as_str(), expected);
        }
        other => panic!("expected filename mismatch, got {other:?}"),
    }
}

#[tokio::test]
async fn ls_sorts_and_skips_non_files() {
    let dir = TempDir::new().expect("tempdir");
    let store = open(&dir).await;
    let alpha = trait_with("alpha");
    let beta = trait_with("beta");
    store.add(beta.clone()).await.expect("adds beta");
    store.add(alpha.clone()).await.expect("adds alpha");

    std::fs::write(dir.path().join("notes.txt"), "not a trait").expect("writes txt");
    std::fs::create_dir(dir.path().join("gamma.md")).expect("creates directory");

    let listed = store.ls().await.expect("lists");
    assert_eq!(listed, vec![alpha, beta]);
}

#[tokio::test]
async fn parse_failure_aborts_ls() {
    let dir = TempDir::new().expect("tempdir");
    let store = open(&dir).await;
    store.add(trait_with("aaa")).await.expect("adds");
    std::fs::write(file(&dir, "zzz"), "---\nname: [\n---\nBody").expect("writes invalid");

    let failure = store.ls().await.expect_err("ls fails on parse error");
    assert!(matches!(failure, ToolFailure::TraitParseError { .. }));
}

#[tokio::test]
async fn parse_errors_report_recovery_and_do_not_mutate() {
    let dir = TempDir::new().expect("tempdir");
    let store = open(&dir).await;
    let trait_value = trait_with("old-format");
    let path = file(&dir, "old-format");

    let fixtures = [
        (
            "---\nname: old-format\n---\nPreserve this rule.",
            "does not match the installed Trait schema",
        ),
        (
            "---\nname: [\n---\nPreserve this rule.",
            "do not assume this is a schema migration",
        ),
    ];

    for (text, diagnostic) in fixtures {
        std::fs::write(&path, text).expect("writes fixture");

        let operations: Vec<Result<(), ToolFailure>> = vec![
            store.ls().await.map(|_| ()),
            store.get(&trait_value.name).await.map(|_| ()),
            store.update(trait_value.clone()).await,
            store.rm(std::slice::from_ref(&trait_value.name)).await,
        ];
        for result in operations {
            let failure = result.expect_err("operation fails");
            assert!(matches!(failure, ToolFailure::TraitParseError { .. }));
            assert!(
                failure.message().contains(diagnostic),
                "{}",
                failure.message()
            );
            for substring in RECOVERY_SUBSTRINGS {
                assert!(
                    failure.message().contains(substring),
                    "missing {substring:?} in {}",
                    failure.message()
                );
            }
        }

        assert_eq!(read(&path), text, "file must be untouched");
    }
}

#[tokio::test]
async fn rm_stops_at_the_first_error_but_keeps_earlier_removals() {
    let dir = TempDir::new().expect("tempdir");
    let store = open(&dir).await;
    store.add(trait_with("first")).await.expect("adds first");
    store.add(trait_with("second")).await.expect("adds second");

    let failure = store
        .rm(&[name("first"), name("absent"), name("second")])
        .await
        .expect_err("rm fails");
    assert!(matches!(failure, ToolFailure::TraitNotFoundError { .. }));
    assert!(!file(&dir, "first").exists(), "earlier removal persists");
    assert!(file(&dir, "second").exists(), "later removals do not run");
}
