# Check through MCP

The MCP server exposes `check_traits` over stdio. An agent initializes the
server, lists the two tools, asks it to assess selected files, and receives
either a JSON finding array or a structured tool failure. Configuration
failures keep the server connected and return a diagnostic from the tool.

## Sub-features

- `mcp-handshake` initializes the server and reads its instructions.
- `mcp-tools` lists exactly `derive_trait` and `check_traits`.
- `mcp-check` returns the same finding shape as the CLI for a violating file.
- `mcp-empty` distinguishes an empty include selection from an assessment.
- `mcp-errors` returns `TraitNotFoundError` for an unknown focused trait.
- `mcp-config-failure` keeps the server alive and reports a missing provider
  field from `check_traits`.

## How to get to it (user POV)

- Add the server to the agent's MCP configuration:
  `{"mcp":{"servers":{"traits":{"type":"local","command":["traits"]}}}}`.
  This repository's `opencode.json` already contains that entry.
- Let the agent call `check_traits({ include: [...] })` after editing files.
- Use `traitNames` to focus a check and `circumstances` to describe the
  triggering task.

## Driving it with control-traits

Preconditions:

- The doctor passes for this run ID.
- The fixture has `use-const` scoped to `*.ts`, plus `example.ts` (`let`),
  `good.ts` (`const`), and `ignored.ts` (a `use-const` ignore directive).
- The local mock provider is running; the harness starts it.
- The MCP client sends empty capabilities, so this recipe exercises the
  non-form fallback path, not native elicitation.

- **Initialize.** Run
  `./.opencode/skills/verify-traits/control-traits run mcp-check --run-id <id>`.
  `mcp-check/summary.json` has `MCP initialize names the traits server` and
  `MCP initialize includes instructions` true.
- **List tools.** The same run checks `MCP exposes exactly two tools`; the
  transcript's `tools/list` response contains `derive_trait` and
  `check_traits`.
- **Assess files.** The same run checks `MCP check_traits returns one finding`;
  `mcp-check/findings.json` contains `trait.name == "use-const"` and
  `source.location.startLine == 1`.
- **Prove an empty selection.** The same run checks `MCP empty include returns
  []`; `check_traits({ include: [] })` returns `[]` without a model request.
- **Respect an ignore directive.** The same run checks `MCP ignore directive
  returns []`.
- **Surface an unknown trait.** The same run checks `MCP unknown trait is an
  error`; the tool result has `isError: true` and `TraitNotFoundError`.
- **Survive a broken config.** The same run starts a second server whose
  provider lacks `base-url`; it checks `MCP starts even when configuration
  fails` and `a broken configuration returns a diagnostic` containing
  `base-url`.
- **Proof.** Read `mcp-check/mcp-check.jsonl`,
  `mcp-check/mcp-check-broken.jsonl`, and `mcp-check/findings.json`.

## Gotchas

- Server stdout is JSON-RPC only; logs go to stderr. Do not parse a mixed
  stream.
- Send `initialize` and then `notifications/initialized` before calling tools.
  The harness uses protocol version `2025-11-25`.
- A configuration failure does not stop the server. `initialize` and
  `tools/list` still succeed; `check_traits` returns `isError: true` with the
  diagnostic and recovery instructions.
- An empty `include` array and an unmatched pattern both select no files and
  return `[]`. They are not proof that a file was assessed.
- An unknown `traitNames` entry is a tool error, not an empty result.
- The harness uses empty client capabilities. Native MCP form elicitation is
  not exercised here; `mcp-derive` covers the fallback approval path.
- The mock provider is a local OpenAI-compatible endpoint. The MCP transport,
  tool dispatch, config loader, file discovery, and finding serialization are
  real.
