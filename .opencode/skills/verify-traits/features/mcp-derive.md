# Derive a trait through MCP

`derive_trait` takes a complete proposed trait, checks it against the stored
traits, and either asks for approval, saves a new trait, updates a covered or
clarified rule, or enters reconciliation. The harness exercises the distinct
creation path: no approval means no write, approval creates the file, and the
CLI can read it back.

## Sub-features

- `mcp-derive-approval` returns `ApprovalRequired` and writes nothing when
  `approved` is absent.
- `mcp-derive-create` saves the proposed trait when `approved: true`.
- `mcp-derive-round-trip` preserves frontmatter, `paths`, and Markdown context
  on disk.
- `mcp-derive-duplicate` rejects a second trait with the same canonical name.
- `mcp-derive-invalid` rejects a proposal missing a required field without
  writing.

## How to get to it (user POV)

- Ask the agent to remember a convention; the agent calls `derive_trait` with a
  complete `proposedTrait`.
- On `ApprovalRequired`, approve the exact proposal and retry with
  `approved: true`.
- Inspect the saved trait with `traits trait show <name>`.

## Driving it with control-traits

Preconditions:

- The doctor passes for this run ID.
- The fixture has an empty `.agents/traits/` store and a mock provider config.
- The MCP client sends empty capabilities, so the fallback approval path is
  used instead of a native form.

- **Propose without approval.** Run
  `./.opencode/skills/verify-traits/control-traits run mcp-derive --run-id <id>`.
  `mcp-derive/summary.json` has `derive without approval does not write` and
  `derive without approval asks for approval`; the result is `_tag:
  ApprovalRequired` with `Save trait prefer-const?`.
- **Approve and create.** The same run checks `deriving returns Created` and
  `deriving writes the trait file`; `mcp-derive/fixtures/prefer-const.md`
  exists.
- **Prove the round trip.** The same run checks `the saved file keeps paths`
  and `the saved file keeps context`; the file contains the `*.ts` and `*.rs`
  globs and the `BAD:`/`GOOD:` examples.
- **Read it from the CLI.** The same run checks `the CLI sees the MCP-created
  trait`; `traits trait show prefer-const` exits `0` and prints the same
  context.
- **Reject a duplicate.** The same run checks `a duplicate name is an error`;
  the tool returns `TraitAlreadyExistsError`.
- **Reject an invalid proposal.** The same run checks `an invalid proposal is
  an error` and `an invalid proposal writes nothing`; a proposal missing
  `purpose` returns `Invalid proposedTrait`.
- **Proof.** Read `mcp-derive/mcp-derive.jsonl`, `mcp-derive/derive.json`, and
  `mcp-derive/fixtures/prefer-const.md`.

## Gotchas

- `derive_trait` without `approved` is a read-only proposal check. Do not treat
  `ApprovalRequired` as a save, and do not retry with `approved: true` unless
  the exact proposal was approved.
- The harness uses an empty store and a mock that answers `distinct`. It does
  not exercise `Covered`, `Clarification`, `Updated`, or `Resolved`.
- A duplicate canonical name is an error even if the proposed body differs.
  Renaming is add-then-remove, not an in-place identity change.
- `paths` is a list of worktree-relative globs; a bare directory is not
  expanded. Put file types and locations in `paths`, not in applicability.
- The mock provider is only called for the duplicate check. Creation itself
  does not need a language model.
- The first view is the MCP result; the second view is `traits trait show`.
  Both are required to prove persistence.
