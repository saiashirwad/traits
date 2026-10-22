# Project Guidelines

All paths below are relative to the repository root.

## Workspace

- This is a single Rust crate. `traits` is both the CLI and the MCP stdio
  server.
- `src/` holds the CLI, config, store, provider adapters, batching, cache,
  conflict/derive logic, and MCP server.
- `src/prompts/` holds MCP instructions and tool descriptions.
- `skills/traits/` is the bundled agent skill. `.agents/skills/traits` is a
  symlink to it; edit the bundled skill, not the symlink.
- `.agents/traits/` is this repository's trait store; `traits.kdl` selects
  providers and models.
- This checkout is Rust-only. Do not reintroduce the retired pnpm/Effect
  workspace or the static site.

## Verification

- Build the checkout binary: `cargo build --bin traits`.
- Run the CI checks locally: `cargo fmt --check`, `cargo clippy --all-targets
  -- -D warnings`, `cargo test`.
- Use `target/debug/traits` for manual CLI and MCP work; do not rely on a
  globally installed `traits`.
- For user-facing CLI or MCP changes, run the project-local verification skill:
  `./.opencode/skills/verify-traits/control-traits doctor --run-id <id>`, then
  `... run <feature> --run-id <id>` or `... run all --run-id <id>`.
- Tests must not call live model providers. Integration tests use `wiremock`;
  the verification skill uses a local mock provider for protocol and CLI
  plumbing. Neither claims model quality; a real-model check needs an explicit
  opt-in and credentials.
- Report failures, blocked checks, and skipped suites accurately. An empty test
  selection is not a behavior-test pass.

## Configuration and credentials

- `traits.kdl` names providers and models; keys come from the process
  environment or a Git-ignored `.env`.
- Never ask for secret values in chat, print them, or commit them.
- `traits config check --live` sends one request per configured role. Use it
  only with credentials you are authorized to spend.
- `traits check` sends source text and trait bodies to the configured classify
  provider.

## Trait work

- Store traits in `.agents/traits/<name>.md`; required fields are `name`,
  `subject`, `requirement`, and `purpose`.
- Use `traits trait ls/show/rm` to inspect the store.
- After creating or changing a trait, run `check_traits` against eligible
  project files with the saved trait names.

## Enforcement

| Rule | Enforced by |
| --- | --- |
| Formatting, lint, and tests pass | `.github/workflows/rust.yml` runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` |
| The checkout binary builds | `.github/workflows/rust.yml` runs `cargo build --bin traits` |
| The verification skill stays runnable | CI compiles `control-traits` and checks that `SKILL.md` and the feature map exist; run the map locally for behavior proof |
| Model calls stay out of tests | `wiremock` in `tests/`; the `control-traits` mock provider is opt-in and covers protocol/CLI plumbing only |
| Use the Rust crate, not the retired pnpm workspace | CI runs only Cargo and the verification script; no pnpm workspace exists |
