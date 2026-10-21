use std::path::Path;
use std::process::Stdio;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

struct McpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
}

impl McpClient {
    async fn start(cwd: &Path, env: &[(&str, &str)]) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_traits"));
        command
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        for (key, value) in env {
            command.env(key, value);
        }
        let mut child = command.spawn().expect("spawn traits");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        Self {
            child,
            stdin,
            stdout,
            next_id: 1,
        }
    }

    async fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let message = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        let mut line = serde_json::to_string(&message).unwrap();
        line.push('\n');
        self.stdin.write_all(line.as_bytes()).await.unwrap();
        self.stdin.flush().await.unwrap();
        loop {
            let mut response = String::new();
            let read = self.stdout.read_line(&mut response).await.unwrap();
            assert!(read > 0, "server closed stdout while waiting for {method}");
            let value: Value =
                serde_json::from_str(&response).expect("server sent non-JSON output");
            if value.get("id").and_then(Value::as_i64) == Some(id) {
                if let Some(error) = value.get("error") {
                    panic!("{method} failed: {error}");
                }
                return value.get("result").cloned().unwrap_or(Value::Null);
            }
        }
    }

    async fn notify(&mut self, method: &str, params: Value) {
        let message = json!({"jsonrpc": "2.0", "method": method, "params": params});
        let mut line = serde_json::to_string(&message).unwrap();
        line.push('\n');
        self.stdin.write_all(line.as_bytes()).await.unwrap();
        self.stdin.flush().await.unwrap();
    }

    async fn initialize(&mut self) -> Value {
        let result = self
            .request(
                "initialize",
                json!({
                    "protocolVersion": "2025-11-25",
                    "capabilities": {},
                    "clientInfo": {"name": "traits-e2e", "version": "0"}
                }),
            )
            .await;
        self.notify("notifications/initialized", json!({})).await;
        result
    }

    async fn call(&mut self, name: &str, arguments: Value) -> Value {
        let result = self
            .request("tools/call", json!({"name": name, "arguments": arguments}))
            .await;
        let text = result["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        serde_json::from_str(&text).unwrap_or_else(|_| json!(text))
    }

    async fn stop(mut self) {
        drop(self.stdin);
        let _ = self.child.wait().await;
    }
}

fn write_fixture(dir: &Path) {
    std::fs::create_dir_all(dir.join(".agents/traits")).unwrap();
    std::fs::write(
        dir.join(".agents/traits/use-const.md"),
        "---\nname: use-const\nsubject: Binding declarations.\nrequirement: Prefer const declarations.\npurpose: Make immutable bindings explicit.\n---\n",
    )
    .unwrap();
    std::fs::write(dir.join("example.ts"), "let value = 1\n").unwrap();
    std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(dir)
        .status()
        .unwrap();
}

fn write_config(dir: &Path, base_url: &str) {
    std::fs::write(
        dir.join("traits.kdl"),
        format!(
            "provider \"mock\" {{\n    kind \"custom\"\n    model \"mock-model\"\n    base-url \"{base_url}\"\n    api-key-env \"MOCK_API_KEY\"\n}}\ndefault-provider \"mock\"\nexclude-gitignored #false\n"
        ),
    )
    .unwrap();
}

fn classification_response(label: &str, trait_name: &str) -> ResponseTemplate {
    let content = json!({
        "answers": [{
            "id": trait_name,
            "label": label,
            "probabilities": {
                "compliant": if label == "compliant" { 1.0 } else { 0.0 },
                "violation": if label == "violation" { 1.0 } else { 0.0 },
                "not-applicable": 0.0,
                "inconclusive": 0.0
            }
        }]
    });
    ResponseTemplate::new(200).set_body_json(json!({
        "choices": [{"message": {"content": content.to_string()}}],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1}
    }))
}

#[tokio::test]
async fn serves_tools_and_runs_a_check_over_stdio() {
    let dir = tempfile::tempdir().unwrap();
    write_fixture(dir.path());
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(classification_response("violation", "use-const"))
        .mount(&server)
        .await;
    write_config(dir.path(), &server.uri());

    let mut client = McpClient::start(dir.path(), &[("MOCK_API_KEY", "test")]).await;
    let init = client.initialize().await;
    assert_eq!(init["serverInfo"]["name"], "traits");
    assert!(init["instructions"].as_str().unwrap().contains("Traits"));

    let tools = client.request("tools/list", json!({})).await;
    let names: Vec<&str> = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["derive_trait", "check_traits"]);

    let empty = client.call("check_traits", json!({"include": []})).await;
    assert_eq!(empty, json!([]));

    let findings = client
        .call("check_traits", json!({"include": ["example.ts"]}))
        .await;
    let finding = &findings[0];
    assert_eq!(finding["trait"]["name"], "use-const");
    assert_eq!(finding["assessment"]["classification"], "violation");
    assert_eq!(finding["source"]["location"]["startLine"], 1);

    client.stop().await;
}

#[tokio::test]
async fn derive_trait_creates_after_fallback_approval() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".agents/traits")).unwrap();
    std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    write_config(dir.path(), &server.uri());

    let mut client = McpClient::start(dir.path(), &[("MOCK_API_KEY", "test")]).await;
    client.initialize().await;

    let result = client
        .call(
            "derive_trait",
            json!({
                "proposedTrait": {
                    "name": "arrow-functions",
                    "subject": "Ordinary functions.",
                    "requirement": "Use arrow functions.",
                    "purpose": "Keep function syntax consistent."
                },
                "approved": true
            }),
        )
        .await;
    assert_eq!(result["_tag"], "Created");
    assert!(
        dir.path()
            .join(".agents/traits/arrow-functions.md")
            .exists()
    );

    client.stop().await;
}

#[tokio::test]
async fn stays_connected_when_configuration_fails() {
    let dir = tempfile::tempdir().unwrap();
    std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    std::fs::write(
        dir.path().join("traits.kdl"),
        "provider \"broken\" {\n    kind \"custom\"\n    model \"m\"\n}\n",
    )
    .unwrap();

    let mut client = McpClient::start(dir.path(), &[]).await;
    let init = client.initialize().await;
    assert_eq!(init["serverInfo"]["name"], "traits");

    let tools = client.request("tools/list", json!({})).await;
    assert_eq!(tools["tools"].as_array().unwrap().len(), 2);

    let result = client
        .request(
            "tools/call",
            json!({"name": "check_traits", "arguments": {"include": []}}),
        )
        .await;
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("base-url"), "unexpected diagnostic: {text}");
    assert!(result["isError"].as_bool().unwrap());

    client.stop().await;
}
