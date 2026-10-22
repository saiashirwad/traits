---
name: verify-traits
description: >-
  Drive the Traits Rust CLI and MCP server end to end with disposable
  fixtures and a local mock provider. Use after changing the CLI, config
  resolution, trait store, check/cache behavior, MCP tools, or provider
  plumbing, and before claiming a user-facing path works.
---

# Verify Traits

Traits has two user surfaces: the `traits` CLI and the `traits` MCP stdio
server. `control-traits` drives both against a throwaway Git fixture and a
local OpenAI-compatible mock. It never calls a real provider and never writes
into the user's trait store.

This is an opt-in verification tool, not a unit-test suite. It proves protocol,
CLI, config, store, and failure-handling behavior; it does not claim model
quality. Model-dependent behavior needs a separate opt-in real-provider check.

## Launch

Build the checkout's binary once:

```sh
cargo build --bin traits
./target/debug/traits --version
```

Expected: `traits <version>` matching `Cargo.toml`. `control-traits` uses
`target/debug/traits` by default; override with `TRAITS_BIN` only when
deliberately testing another build.

There is no long-running server. Each `run` creates a fixture under
`$TMPDIR/traits-verify/<run-id>/`, starts a mock provider on a loopback port,
drives the real binary, and stops everything before returning.

## Doctor

```sh
./.opencode/skills/verify-traits/control-traits doctor --run-id <run-id>
```

The doctor checks the binary exists, matches `Cargo.toml`, is newer than
`src/`, and that Python 3.9+ and Git are available. It then creates a
disposable fixture, runs `traits config check --live`, `traits check`,
`traits trait ls`, and an MCP `initialize`/`tools/list` handshake. It removes
the fixture and writes `artifacts/verify-traits/<run-id>/doctor/summary.json`.

Run the doctor first whenever a drive fails, the binary may be stale, or a
previous run left state behind. If it fails, fix the reported prerequisite; do
not drive.

## Drive

Run one mapped feature or the full map:

```sh
./.opencode/skills/verify-traits/control-traits run check --run-id <run-id>
./.opencode/skills/verify-traits/control-traits run mcp-check --run-id <run-id>
./.opencode/skills/verify-traits/control-traits run all --run-id <run-id>
```

`run all` runs the doctor plus every feature. Use a fresh run ID per session;
reusing one replaces that feature's evidence. Each feature's `summary.json`
lists every assertion, and `transcript.jsonl` records each command's argv, exit
code, stdout, and stderr. Feature recipes live in
[`features/README.md`](./features/README.md).

Do not start `traits serve` by hand, kill processes by name, or point a drive
at a real `.agents/traits` store. The harness starts and stops every MCP child
and mock server it owns.

## Evidence

Proof lives under `artifacts/verify-traits/<run-id>/`:

- `summary.json` is the aggregate pass/fail result.
- `<feature>/summary.json` lists each check and its observed output.
- `<feature>/transcript.jsonl` holds every CLI command and MCP frame.
- `mcp-check/mcp-check.jsonl` and `mcp-derive/mcp-derive.jsonl` are the raw MCP
  transcripts.
- `mcp-derive/fixtures/prefer-const.md` is the trait written through MCP;
  `mcp-derive/derive.json` includes the second-view CLI output.
- `check/findings.json` is the parsed `--json` finding.

The proof standard is the real path: build the checkout binary, run the CLI or
MCP server, observe the exit code/stdout/stderr or JSON-RPC result, and verify
mutations from a second view. The only mocked boundary is the model provider.
`check` uses a local HTTP server implementing `/chat/completions`; the CLI, MCP
transport, config loader, file discovery, chunking, cache, and store all run
for real. The mock answers violation for `let`/`var` and compliant otherwise,
so it proves plumbing, not model judgment.

A run is not proof if it only reads `"ok": true`. Inspect the named artifact
and the command output that supports the claim. If a path is unreachable,
report the attempted command and the unmet precondition; do not report it as
verified through a different path.

## Cleanup

```sh
./.opencode/skills/verify-traits/control-traits cleanup --run-id <run-id>
./.opencode/skills/verify-traits/control-traits cleanup --all
```

Cleanup removes only `$TMPDIR/traits-verify/<run-id>` (or the whole fixture
root). It never removes `artifacts/verify-traits/`, so evidence survives. The
harness stops its own mock server and MCP child in `finally`; cleanup is for
fixture directories left by an interrupted run. Never kill `traits` by process
name.

## Helpers

- [`control-traits`](./control-traits) is the executable harness. It has
  `doctor`, `run <feature|all>`, and `cleanup` subcommands;
  `python3 ... --help` prints usage.
- [`features/README.md`](./features/README.md) is the maintained feature map.
  Read it before driving; use the matching feature file as the recipe.
- The harness requires only Python 3.9+ and Git beyond the crate's own build.
