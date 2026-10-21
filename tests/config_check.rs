use std::path::Path;
use std::process::Output;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn write_config(dir: &Path, base_url: &str) {
    std::fs::write(
        dir.join("traits.kdl"),
        format!(
            r#"provider "decisions" {{
    kind "typesafe"
    model "jev-latest"
    base-url "{base_url}"
    api-key-env "MOCK_API_KEY"
}}
provider "chat" {{
    kind "custom"
    model "chat-model"
    base-url "{base_url}"
    api-key-env "MOCK_API_KEY"
}}
classify {{
    provider "decisions"
}}
generate {{
    provider "chat"
}}
"#
        ),
    )
    .unwrap();
}

async fn config_check(dir: &Path, args: &[&str]) -> Output {
    tokio::process::Command::new(env!("CARGO_BIN_EXE_traits"))
        .args(["config", "check"])
        .args(args)
        .current_dir(dir)
        .env("MOCK_API_KEY", "test-key")
        .output()
        .await
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

async fn mount_decision(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "answers": {
                "ping": {
                    "choice": "yes",
                    "probabilities": { "yes": 1.0, "no": 0.0 }
                }
            }
        })))
        .mount(server)
        .await;
}

async fn mount_chat(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(response)
        .mount(server)
        .await;
}

#[tokio::test]
async fn live_check_reports_each_role_and_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    mount_decision(&server).await;
    mount_chat(
        &server,
        ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{ "message": { "content": "{\"ok\":true}" } }]
        })),
    )
    .await;
    write_config(dir.path(), &server.uri());

    let output = config_check(dir.path(), &[]).await;
    assert!(output.status.success());
    assert!(server.received_requests().await.unwrap().is_empty());

    let output = config_check(dir.path(), &["--live"]).await;
    assert!(output.status.success(), "{}", stdout(&output));
    let text = stdout(&output);
    let lines = text
        .lines()
        .filter(|line| line.starts_with("classify ") || line.starts_with("generate "))
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 2, "{text}");
    assert!(lines[0].starts_with("classify decisions (jev-latest): ok in "));
    assert!(lines[1].starts_with("generate chat (chat-model): ok in "));
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn live_check_fails_when_any_role_fails() {
    let dir = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    mount_decision(&server).await;
    mount_chat(&server, ResponseTemplate::new(500).set_body_string("down")).await;
    write_config(dir.path(), &server.uri());

    let output = config_check(dir.path(), &["--live"]).await;
    assert!(!output.status.success());
    let text = stdout(&output);
    assert!(
        text.contains("classify decisions (jev-latest): ok in "),
        "{text}"
    );
    assert!(
        text.contains("generate chat (chat-model): failed: Chat completions returned HTTP 500"),
        "{text}"
    );
}
