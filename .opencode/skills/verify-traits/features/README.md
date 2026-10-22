# Traits verification map

This directory is the maintained source for verifying the user-facing behavior
of Traits. Read this index before driving, then use the matching feature file
as the recipe.

The map is an opt-in verification tool, not a unit-test suite. It proves
protocol, CLI, config, store, and failure-handling behavior with a local mock
provider; it does not claim model quality.

## Baseline preconditions

- Run from the repository root. `control-traits` resolves the root from its own
  path.
- `cargo build --bin traits` has produced `target/debug/traits` newer than
  `src/`.
- `./.opencode/skills/verify-traits/control-traits doctor --run-id <id>` passes.
- The harness sets `MOCK_API_KEY=verify-key` only for fixture subprocesses and
  removes known provider keys from their environment. Never put a real key in a
  fixture.
- Fixtures live under `$TMPDIR/traits-verify/<run-id>`; evidence lives under
  `artifacts/verify-traits/<run-id>`.
- Each fixture is a fresh Git repository. `exclude-gitignored #false` keeps
  checks independent of index state; `HOME` is fixture-local so global config
  and cache cannot leak.
- Never drive an instance that was not started by this verification run.

## Driving conventions

- Start each recipe from a fresh fixture with
  `control-traits run <feature> --run-id <id>`.
- Use one run ID per session. Reusing one replaces that feature's evidence.
- Treat commands as literal. Keep quoted names and flags unchanged.
- Use `target/debug/traits`, not a globally installed `traits`; the doctor
  checks the checkout binary.
- The harness starts the mock provider and MCP child. Do not start
  `traits serve` by hand or kill processes by name.
- Read the feature's `summary.json` and `transcript.jsonl`; do not rely on
  memory or on the aggregate `"ok"` alone.

## Proof and skip reporting

- CLI proof includes the command, exit code, stdout, and stderr.
- MCP proof includes the JSON-RPC transcript and the parsed tool result.
- Mutation proof includes the written file and a read-only second view, such as
  `traits trait show` after MCP `derive_trait`.
- The only mocked boundary is the model provider. The CLI, MCP transport,
  config loader, file discovery, chunking, cache, and store run for real.
- A compliant result is filtered out of `traits check` output. `No findings.`
  alone does not prove a file was assessed; use the mock request count,
  `--json`, or a focused trait to distinguish "assessed compliant" from
  "nothing selected".
- Report an unreachable path with the attempted command and the unmet
  precondition. Do not report a skipped entry point as verified through a
  different path.

## Feature entry contract

Each feature file starts with an H1 title and one paragraph describing the
user-visible behavior. It then uses exactly four H2 sections in this order.

1. `Sub-features` lists short IDs with one line for each behavior.
2. `How to get to it (user POV)` lists every user entry point.
3. `Driving it with control-traits` starts with `Preconditions:` and uses
   labeled bullets that pair each user action with an exact command and
   observable result.
4. `Gotchas` lists traps that can waste or invalidate a verification run.

Keep implementation details out of the map. Name only user paths, stable
handles, required state, commands, and observable proof.

## Features

- [Set up a repository](./setup.md) covers `traits init`, overwrite
  protection, and `traits config check` with and without `--live`.
- [Inspect the trait store](./store.md) covers `traits trait ls`, `show`, `rm`,
  and parse failures.
- [Check project files](./check.md) covers patterns, focused traits, JSON,
  ignore directives, chunking, and the answer cache.
- [Check through MCP](./mcp-check.md) covers the stdio handshake, tool catalog,
  `check_traits`, and configuration failures.
- [Derive a trait through MCP](./mcp-derive.md) covers approval, creation,
  duplicate names, and invalid proposals.

## Not yet mapped

- `traits skill install` and the bundled skill copy.
- `derive_trait` covered, clarification, and reconciliation outcomes.
- Live provider/model behavior and `traits config check --live` against real
  providers.
- Release, installer, and Homebrew paths.
