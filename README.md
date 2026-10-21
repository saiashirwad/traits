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
`applicability` and `guidance` are optional text fields. The Markdown body holds
supporting context, examples, and references. Unknown frontmatter keys are
rejected. Guidance does not grant permission to act, and model assessments are
not guarantees.

The server is one Rust binary. It installs nothing into your project.

## Install

```sh
cargo install --path traits-rs
```

Until the crate is published, install from a clone. The binary lands in
`~/.cargo/bin/traits`.

## Set up a repository

```sh
cd your-repo
traits init             # writes traits.kdl and creates .agents/traits/
traits config check     # validates providers and credentials
```

`traits.kdl` names the providers and models to use. [SETUP.md](SETUP.md) has
provider examples and credential handling.

Add the server to your agent's MCP configuration:

```json
{ "mcp": { "servers": { "traits": { "type": "local", "command": ["traits"] } } } }
```

Install the bundled agent skill:

```sh
traits skill install --agent project
```

## CLI

| Command | Purpose |
|---|---|
| `traits` | MCP stdio server when stdin is piped |
| `traits serve` | MCP stdio server |
| `traits init [--force]` | Starter `traits.kdl` and store |
| `traits check [patterns] [--trait NAME] [--json]` | Assess files; exits 1 on violations |
| `traits trait ls`, `show NAME`, `rm NAME` | Inspect the store |
| `traits config check` | Validate the resolved configuration |
| `traits skill install --agent project\|opencode\|claude\|all` | Write the agent skill |

## Repository layout

| Path | Contents |
|---|---|
| `traits-rs/` | The Rust crate, its prompts, skill, and tests |
| `traits.kdl` | This repository's configuration |
| `.agents/traits/` | This repository's trait store |
| `.agents/skills/traits` | Symlink to the bundled skill |
| `repos/` | Reference checkouts (Git submodules, excluded from checks) |
