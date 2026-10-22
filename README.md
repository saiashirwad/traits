# Traits

Reusable preferences for your AI agent. Your agent proposes traits, asks you to
approve them, and checks its work against them over MCP.

Each trait is a Markdown file in `.agents/traits/<name>.md`:

```markdown
---
name: direct-callbacks
subject: Callbacks passed to functions.
requirement: >
  Pass compatible functions directly instead of wrapping them in forwarding
  arrows.
purpose: Expose callback behavior without redundant wrappers.
---
```

Each trait requires `name`, `subject`, `requirement`, and `purpose`.
`applicability` and `guidance` are optional text fields. `paths` is an optional
list of worktree-relative globs, such as `"*.tsx"`, naming the only files the
trait can apply to; other files are never sent to the model. The Markdown body
holds supporting context, examples, and references. Unknown frontmatter keys are
rejected. Guidance does not grant permission to act, and model assessments are
not guarantees.

The server is one Rust binary. It installs nothing into your project.

## Install

```sh
brew install saiashirwad/tap/traits
```

Without Homebrew, use the installer from the latest release:

```sh
curl -LsSf https://github.com/saiashirwad/traits/releases/latest/download/traits-installer.sh | sh
```

From a clone, run `cargo install --path .`.

## Set up a repository

```sh
cd your-repo
traits init             # writes traits.kdl and creates .agents/traits/
traits config check     # validates providers and credentials
traits config check --live   # sends one tiny request per role
```

`traits.kdl` names the providers and models to use. [SETUP.md](SETUP.md) has
provider examples and credential handling.

## Configuration

Providers and defaults live in `~/.config/traits/config.kdl`. A project can
override them with `traits.kdl` at the worktree root. API keys are read from
the environment variable named by `api-key-env`: the process environment, then
`.env` at the worktree root, then `.env` at the main checkout root.

```kdl
version 1
concurrency 4
exclude "**/dist/**"

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

`check_traits` caches each conclusive answer in the user cache directory
(`~/Library/Caches/traits` on macOS, `~/.cache/traits` on Linux), keyed by a
hash of the provider, the trait's question, and the chunk it assessed. A rerun
asks only about changed files, changed traits, or new circumstances. Turn it
off with `cache #false`, or for one run with `traits check --no-cache`.

Add the server to your agent's MCP configuration:

```json
{ "mcp": { "servers": { "traits": { "type": "local", "command": ["traits"] } } } }
```

Install the bundled agent skill:

```sh
traits skill install --agent project
```

## CLI

| Command                                                                               | Purpose                                   |
| ------------------------------------------------------------------------------------- | ----------------------------------------- |
| `traits`                                                                              | MCP stdio server when stdin is piped      |
| `traits serve`                                                                        | MCP stdio server                          |
| `traits init [--force]`                                                               | Starter `traits.kdl` and store            |
| `traits check [patterns] [--trait NAME] [--circumstances TEXT] [--json] [--no-cache]` | Assess files; exits 1 on violations       |
| `traits trait ls`, `show NAME`, `rm NAME`                                             | Inspect the store                         |
| `traits config check [--live]`                                                        | Validate config; `--live` pings each role |
| `traits skill install --agent project\|opencode\|claude\|all`                         | Write the agent skill                     |

## Repository layout

| Path                    | Contents                               |
| ----------------------- | -------------------------------------- |
| `src/`, `tests/`        | The Rust crate and its tests           |
| `src/prompts/`          | MCP instructions and tool descriptions |
| `skills/traits/`        | The bundled agent skill                |
| `traits.kdl`            | This repository's configuration        |
| `.agents/traits/`       | This repository's trait store          |
| `.agents/skills/traits` | Symlink to the bundled skill           |

## Modules

| Module                     | Purpose                                                    |
| -------------------------- | ---------------------------------------------------------- |
| `doc.rs`                   | Trait schema, frontmatter parse and serialize              |
| `store.rs`                 | Filesystem store                                           |
| `config.rs`                | KDL parsing, merge, provider resolution                    |
| `provider/`                | TypeSafe decisions and OpenAI-compatible chat              |
| `prompt.rs`, `batching.rs` | Assessment prompts, batching, `check_traits`               |
| `cache.rs`                 | Content-addressed `check_traits` answers                   |
| `files.rs`                 | `git ls-files`, glob includes, excludes, ignore directives |
| `conflict.rs`, `derive.rs` | Relationship checks, derive and reconcile                  |
| `mcp.rs`                   | MCP tools, approval elicitation, fallback diagnostics      |
| `commands.rs`, `main.rs`   | CLI                                                        |
