# Set up Traits

Traits is configured by your coding agent. Follow these instructions inside the
consumer's Git checkout.

## 1. Install the binary

Install once per machine:

```sh
brew install saiashirwad/tap/traits
```

Without Homebrew:

```sh
curl -LsSf https://github.com/saiashirwad/traits/releases/latest/download/traits-installer.sh | sh
```

From a clone instead:

```sh
git clone https://github.com/saiashirwad/traits ~/.cache/traits
cargo install --path ~/.cache/traits
```

Confirm it with `traits --version`.
The project itself needs no Node, npm, or package installs.

## 2. Ask about providers and models

Ask the user which providers and models to use. Traits has two model roles:

- `classify` answers relationship and assessment questions. TypeSafe returns
  calibrated probability distributions; any OpenAI-compatible chat model also
  works.
- `generate` produces replacement traits during reconciliation. It needs
  schema-constrained JSON output.

Write `traits.kdl` at the worktree root, or `~/.config/traits/config.kdl` for a
machine-wide default. Project values override global values. Ask before
replacing an existing `traits.kdl` and preserve unrelated settings.

TypeSafe decisions with OpenCode Go generation:

```kdl
version 1

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

classify { provider "typesafe" }
generate { provider "opencode" }
```

DeepSeek for both roles:

```kdl
version 1

provider "deepseek" {
    kind "deepseek"
    model "deepseek-flash"
    api-key-env "DEEPSEEK_API_KEY"
}

default-provider "deepseek"
```

Provider kinds: `typesafe`, `opencode-go`, `deepseek`, `openrouter`, `openai`,
`custom`. A custom provider requires `base-url`. Optional provider nodes are
`json-mode` (`schema`, `object`, `prompt`), `reasoning-effort`, `timeout-ms`,
and `header "name" "value"`. `classify` and `generate` accept `provider`,
`temperature`, and `max-tokens`. Set `exclude` nodes to skip files, and
`exclude-gitignored #false` to disable the Git file filter.

## 3. Keep credentials private

Each provider names the environment variable holding its key. Use the MCP
process environment or a Git-ignored `.env` at the checkout root. Never ask for
secret values in chat, print them, or commit them.

Precedence: process environment, then the worktree `.env`, then the main
checkout's `.env`. Reconnect the server after changing credentials.

## 4. Install the skill

```sh
traits skill install --agent project
```

For other agents use `--agent opencode`, `--agent claude`, or `--agent all`.
The skill teaches the agent when to propose traits and how to run checks.

## 5. Connect and verify

Add the server to the agent's MCP configuration, preserving other servers:

```json
{ "mcp": { "servers": { "traits": { "type": "local", "command": ["traits"] } } } }
```

Run `traits config check` from the worktree root. It prints the resolved
providers and warns about missing keys. Add `--live` to send one tiny request
per role and print its latency or error.

When credentials are available, connect the server and confirm `derive_trait`
and `check_traits` are exposed. Do not create a trait merely to test the
installation.

## Troubleshooting

### Configuration failures

The server starts even when configuration or providers fail. Both tools stay
listed and return the diagnostic plus recovery instructions. Fix the reported
error, restart the server, and retry with the same arguments.

### Stored trait parse failures

Tools return `TraitParseError` with the path and diagnostic instead of stopping
the server. Ask the developer before repairing files, and preserve names,
requirements, applicability, examples, and context. Retry after fixing the
files; no restart is needed.

### Reconciliation fails

Reconciliation uses the configured `generate` model with schema-constrained
JSON. A failure usually means the provider returned plain text instead of JSON.
Check `json-mode`: `object` appends the schema to the prompt and works with
DeepSeek-backed models; `schema` uses native `response_format`. OpenCode Go
rejects `json_schema` for its DeepSeek models, so `object` is the default there.

### Checks return nothing

An empty result means no failures for the selected files and traits. Check the
selection: unmatched patterns select no files, an empty trait store checks
nothing, and Git-ignored files are excluded by default.
