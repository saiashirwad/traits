# traits (Rust)

Rust rewrite of the Traits MCP server. One global binary, no per-project
runtime: trait files stay in `.agents/traits/<name>.md` and configuration is
declarative KDL.

## Build and install

```sh
cargo install --path traits-rs          # from this repository
cargo build --release --manifest-path traits-rs/Cargo.toml
```

The binary is `traits`.

## Configuration

Providers and defaults live in `~/.config/traits/config.kdl`; a project can
override them with `traits.kdl` at the worktree root. API keys are read from
the environment variable named by `api-key-env` (process env, then `.env` at
the worktree root, then `.env` at the main checkout root).

```kdl
version 1
concurrency 4
exclude "repos/**"

provider "typesafe" {
    kind "typesafe"
    model "jev-latest"
    api-key-env "TYPESAFE_API_KEY"
}

provider "opencode" {
    kind "opencode-go"
    model "deepseek-v4.1-flash"
    api-key-env "OPENCODE_API_KEY"
}

classify {
    provider "typesafe"
}

generate {
    provider "opencode"
}
```

Provider kinds: `typesafe`, `opencode-go`, `deepseek`, `openrouter`, `openai`,
`custom` (requires `base-url`). Optional provider nodes: `base-url`,
`api-key-env`, `json-mode` (`schema`, `object`, `prompt`), `reasoning-effort`,
`timeout-ms`, `header "name" "value"`. `classify` and `generate` accept
`provider`, `temperature`, and `max-tokens`.

`traits init` writes a starter `traits.kdl` and creates the store.

## CLI

```text
traits                              MCP stdio server when stdin is piped
traits serve                        explicit MCP server
traits init [--force]               starter config + store
traits check [patterns] [--trait NAME] [--circumstances TEXT] [--json]
traits trait ls|show NAME|rm NAME   inspect the store
traits config check                 validate resolved configuration
traits skill install --agent project|opencode|claude|all
```

Add to an agent's MCP configuration:

```json
{ "mcp": { "servers": { "traits": { "type": "local", "command": ["traits"] } } } }
```

## Layout

| Module | Purpose |
|---|---|
| `doc.rs` | trait schema, frontmatter parse/serialize |
| `store.rs` | filesystem store |
| `config.rs` | KDL parsing, merge, env resolution |
| `provider/` | TypeSafe decisions and OpenAI-compatible chat |
| `prompt.rs`, `batching.rs` | assessment prompts, batching, `check_traits` |
| `files.rs` | `git ls-files`, glob includes, ignore directives |
| `conflict.rs`, `derive.rs` | relationship checks, derive/reconcile |
| `mcp.rs` | MCP tools, approval elicitation, fallback diagnostics |
| `commands.rs`, `main.rs` | CLI |
