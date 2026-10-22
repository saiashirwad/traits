# Set up a repository

Setup creates a starter `traits.kdl`, creates `.agents/traits/`, protects an
existing config from accidental overwrite, and validates the resolved
providers and credentials. `--live` sends one tiny request per configured role.

## Sub-features

- `setup-init` writes a starter `traits.kdl` and creates the trait store.
- `setup-init-guard` refuses to overwrite an existing config without `--force`.
- `setup-config` resolves providers and warns about a missing key without
  sending a request.
- `setup-config-live` sends one classify request and one generate request and
  reports their latency.

## How to get to it (user POV)

- Run `traits init` in a Git checkout.
- Run `traits init --force` to replace an existing starter config.
- Run `traits config check` to print the resolved root, store, cache, and
  providers.
- Run `traits config check --live` to ping each configured role.

## Driving it with control-traits

Preconditions:

- The doctor passes for this run ID.
- The fixture is a fresh Git repository with no `traits.kdl`.
- The local mock provider is running; the harness starts it.

- **Initialize a checkout.** Run
  `./.opencode/skills/verify-traits/control-traits run setup --run-id <id>`.
  `setup/summary.json` has `init exits 0`, `init writes traits.kdl`, and
  `init creates the store` all true. `setup/fixtures/traits.kdl` exists.
- **Protect an existing config.** The same run checks
  `second init refuses to overwrite`; `setup/setup.json` shows the second
  command exited `1` with `already exists; pass --force`.
- **Force replacement.** The same run checks `init --force exits 0`.
- **Resolve without network.** The same run checks
  `config check without --live sends no request` and
  `config check warns about the missing key`; `setup/setup.json` contains the
  warning line and no mock request was made.
- **Ping both roles.** The same run checks `live config check makes two
  requests`; stdout has `classify mock (verify-model): ok in` and
  `generate mock (verify-model): ok in`.
- **Proof.** Read `artifacts/verify-traits/<id>/setup/summary.json`. Every
  check is true and `setup/setup.json` carries the raw command output.

## Gotchas

- `traits init` only writes a starter config; it does not validate providers,
  models, or credentials.
- `traits config check` without `--live` makes no network request. A missing-key
  warning is not a connectivity failure.
- `--live` spends one real request per role when pointed at a real provider.
  This harness points it at the local mock; never use a production key.
- `traits init` writes at the Git worktree root, not the current subdirectory.
- `--force` is destructive to the existing `traits.kdl`; use it only in a
  fixture.
