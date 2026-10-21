use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rmcp::ServiceExt;
use serde_json::json;

use crate::batching::check_traits;
use crate::config::{self, ProviderConfig};
use crate::doc::TraitName;
use crate::error::ToolFailure;
use crate::mcp::{Ready, ServerState, TraitsServer};
use crate::provider::{self, Choice, Criterion, GenerateRequest};
use crate::store::{FileSystemStore, TraitStore};

const STARTER_CONFIG: &str = r#"version 1

// Providers define endpoints, models, and the environment variable holding the API key.
provider "opencode" {
    kind "opencode-go"
    model "deepseek-v4.1-flash"
    // api-key-env "OPENCODE_API_KEY"
}

// Use one provider for both classification and generation.
default-provider "opencode"

// Optional project overrides:
// concurrency 4
// exclude-gitignored #true
// cache #true
// exclude "**/dist/**"
"#;

const SKILL: &str = include_str!("../skills/traits/SKILL.md");

pub fn cwd() -> Result<PathBuf, String> {
    std::env::current_dir()
        .map_err(|error| format!("Unable to read the current directory: {error}"))
}

pub fn load_dotenv(root: &Path) {
    let _ = dotenvy::from_path(root.join(".env"));
    let common_dir = std::process::Command::new("git")
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .current_dir(root)
        .output();
    if let Ok(output) = common_dir
        && output.status.success()
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty()
            && let Some(main_root) = Path::new(&path).parent()
            && main_root != root
        {
            let _ = dotenvy::from_path(main_root.join(".env"));
        }
    }
}

pub async fn build_state(config_path: Option<&Path>) -> Result<Ready, String> {
    let cwd = cwd()?;
    let config = config::load(&cwd, config_path).map_err(|error| format!("{error}"))?;
    load_dotenv(&config.config_root);
    let models = provider::build(&config).map_err(|error| format!("{error}"))?;
    let store = FileSystemStore::open(config.store.clone())
        .await
        .map_err(|error| format!("{error}"))?;
    Ok(Ready {
        config,
        store,
        models,
    })
}

pub async fn serve(config_path: Option<&Path>) -> Result<(), String> {
    let state = match build_state(config_path).await {
        Ok(ready) => ServerState::Ready(Arc::new(ready)),
        Err(diagnostic) => ServerState::Failed(Arc::new(diagnostic)),
    };
    let server = TraitsServer::new(state);
    let running = server
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|error| format!("MCP initialization failed: {error}"))?;
    running
        .waiting()
        .await
        .map_err(|error| format!("MCP server failed: {error}"))?;
    Ok(())
}

pub fn init(force: bool) -> Result<(), String> {
    let cwd = cwd()?;
    let root = config::worktree_root(&cwd);
    let path = root.join("traits.kdl");
    if path.exists() && !force {
        return Err(format!(
            "{} already exists; pass --force to overwrite",
            path.display()
        ));
    }
    std::fs::write(&path, STARTER_CONFIG)
        .map_err(|error| format!("Unable to write {}: {error}", path.display()))?;
    std::fs::create_dir_all(root.join(".agents/traits"))
        .map_err(|error| format!("Unable to create the trait store: {error}"))?;
    println!("Wrote {}", path.display());
    println!("Store: {}", root.join(".agents/traits").display());
    println!();
    println!("Add this MCP server to your agent:");
    println!(r#"  "traits": {{ "type": "local", "command": ["traits"] }}"#);
    println!();
    println!("Set OPENCODE_API_KEY (or your provider's key), then run `traits config check`.");
    Ok(())
}

pub async fn check(
    config_path: Option<&Path>,
    patterns: Vec<String>,
    trait_names: Vec<String>,
    circumstances: Option<String>,
    json: bool,
    no_cache: bool,
) -> Result<u8, String> {
    let mut ready = build_state(config_path).await?;
    if no_cache {
        ready.config.cache_dir = None;
    }
    let names = trait_names
        .iter()
        .map(|name| TraitName::parse(name))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let findings = check_traits(
        &ready.config,
        &ready.store,
        &ready.models,
        &patterns,
        if names.is_empty() { None } else { Some(&names) },
        circumstances.as_deref(),
    )
    .await
    .map_err(|failure| failure.message().to_string())?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&findings)
                .map_err(|error| format!("Unable to serialize findings: {error}"))?
        );
    } else if findings.is_empty() {
        println!("No findings.");
    } else {
        for finding in &findings {
            let location = match &finding.source.location {
                Some(crate::doc::SourceLocation::File { path, start_line }) => {
                    format!("{path}:{start_line}")
                }
                _ => "<generated>".to_string(),
            };
            println!(
                "{} {} at {location}",
                finding.assessment.classification.as_str(),
                finding.r#trait.name.as_str()
            );
        }
    }
    let violations = findings
        .iter()
        .filter(|finding| {
            matches!(
                finding.assessment.classification,
                crate::doc::Classification::Violation
            )
        })
        .count();
    Ok(if violations > 0 { 1 } else { 0 })
}

pub async fn trait_ls(config_path: Option<&Path>) -> Result<(), String> {
    let ready = build_state(config_path).await?;
    for trait_value in ready
        .store
        .ls()
        .await
        .map_err(|failure| failure.message().to_string())?
    {
        println!("{} — {}", trait_value.name.as_str(), trait_value.subject);
    }
    Ok(())
}

pub async fn trait_show(config_path: Option<&Path>, name: &str) -> Result<(), String> {
    let ready = build_state(config_path).await?;
    let name = TraitName::parse(name).map_err(|error| error.to_string())?;
    let trait_value = ready
        .store
        .get(&name)
        .await
        .map_err(|failure| failure.message().to_string())?;
    print!("{}", crate::doc::serialize(&trait_value));
    Ok(())
}

pub async fn trait_rm(config_path: Option<&Path>, name: &str) -> Result<(), String> {
    let ready = build_state(config_path).await?;
    let name = TraitName::parse(name).map_err(|error| error.to_string())?;
    ready
        .store
        .rm(std::slice::from_ref(&name))
        .await
        .map_err(|failure| failure.message().to_string())?;
    println!("Removed {}", name.as_str());
    Ok(())
}

pub async fn config_check(config_path: Option<&Path>, live: bool) -> Result<u8, String> {
    let cwd = cwd()?;
    let config = config::load(&cwd, config_path).map_err(|error| format!("{error}"))?;
    load_dotenv(&config.config_root);
    println!("root: {}", config.config_root.display());
    println!("store: {}", config.store.display());
    println!("concurrency: {}", config.concurrency);
    println!("exclude-gitignored: {}", config.exclude_gitignored);
    match &config.cache_dir {
        Some(dir) => println!("cache: {}", dir.display()),
        None => println!("cache: off"),
    }
    if !config.excludes.patterns().is_empty() {
        println!("excludes: {}", config.excludes.patterns().join(", "));
    }
    for provider in &config.providers {
        println!(
            "provider {}: kind={:?} model={} json-mode={:?} key=${}",
            provider.name, provider.kind, provider.model, provider.json_mode, provider.api_key_env
        );
        if std::env::var(&provider.api_key_env)
            .map(|value| value.is_empty())
            .unwrap_or(true)
        {
            println!("  warning: {} is not set", provider.api_key_env);
        }
    }
    if config.providers.is_empty() {
        println!("no providers configured");
    }
    if !live {
        return Ok(0);
    }

    let models = provider::build(&config).map_err(|error| format!("{error}"))?;
    let state = json!({ "text": "ping" });
    let classify = timed(provider::ask::<Ping>(
        models.decision.as_ref(),
        &state,
        "ping",
        "Answer yes when the text is ping.",
    ));
    let generate = async {
        match &models.language {
            Some(language) => Some(
                timed(language.generate_object(GenerateRequest {
                    system: "Reply with JSON only.".to_string(),
                    user: "Return an object whose ok field is true.".to_string(),
                    schema_name: "ping".to_string(),
                    schema: json!({
                        "type": "object",
                        "properties": { "ok": { "type": "boolean" } },
                        "required": ["ok"],
                        "additionalProperties": false
                    }),
                    strict: true,
                }))
                .await,
            ),
            None => None,
        }
    };
    let (classify, generate) = tokio::join!(classify, generate);

    let outcomes = [
        config
            .classify
            .provider
            .as_ref()
            .map(|provider| ("classify", provider, classify)),
        config
            .generate
            .provider
            .as_ref()
            .zip(generate)
            .map(|(provider, outcome)| ("generate", provider, outcome)),
    ];
    let mut failed = false;
    for (role, provider, outcome) in outcomes.into_iter().flatten() {
        failed |= outcome.is_err();
        println!("{}", liveness(role, provider, &outcome));
    }
    Ok(u8::from(failed))
}

#[derive(Debug, Clone, Copy)]
enum Ping {
    Yes,
    No,
}

impl Choice for Ping {
    const ALL: &'static [Self] = &[Self::Yes, Self::No];
    const CRITERIA: &'static [Criterion] = &[
        Criterion {
            label: "yes",
            description: "The text is ping.",
        },
        Criterion {
            label: "no",
            description: "The text is anything else.",
        },
    ];
}

async fn timed<T>(
    request: impl Future<Output = Result<T, ToolFailure>>,
) -> Result<Duration, ToolFailure> {
    let start = Instant::now();
    request.await.map(|_| start.elapsed())
}

fn liveness(
    role: &str,
    provider: &ProviderConfig,
    outcome: &Result<Duration, ToolFailure>,
) -> String {
    let status = match outcome {
        Ok(latency) => format!("ok in {}ms", latency.as_millis()),
        Err(failure) => format!("failed: {}", failure.message()),
    };
    format!("{role} {} ({}): {status}", provider.name, provider.model)
}

pub fn skill_install(agent: &str) -> Result<(), String> {
    let cwd = cwd()?;
    let project_root = config::worktree_root(&cwd);
    let targets: Vec<PathBuf> = match agent {
        "project" => vec![project_root.join(".agents/skills/traits/SKILL.md")],
        "opencode" => vec![
            dirs::config_dir()
                .ok_or_else(|| "Unable to resolve the config directory".to_string())?
                .join("opencode/skills/traits/SKILL.md"),
        ],
        "claude" => vec![
            dirs::home_dir()
                .ok_or_else(|| "Unable to resolve the home directory".to_string())?
                .join(".claude/skills/traits/SKILL.md"),
        ],
        "all" => vec![
            project_root.join(".agents/skills/traits/SKILL.md"),
            dirs::config_dir()
                .ok_or_else(|| "Unable to resolve the config directory".to_string())?
                .join("opencode/skills/traits/SKILL.md"),
            dirs::home_dir()
                .ok_or_else(|| "Unable to resolve the home directory".to_string())?
                .join(".claude/skills/traits/SKILL.md"),
        ],
        other => {
            return Err(format!(
                "Unknown agent \"{other}\"; expected project, opencode, claude, or all"
            ));
        }
    };
    for target in targets {
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("Unable to create {}: {error}", parent.display()))?;
        }
        std::fs::write(&target, SKILL)
            .map_err(|error| format!("Unable to write {}: {error}", target.display()))?;
        println!("Wrote {}", target.display());
    }
    Ok(())
}
