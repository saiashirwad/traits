# Check project files

`traits check` selects project files by path or glob, selects all stored traits
or a focused subset, assesses each 200-line chunk, and prints only findings
that require review. `--json` exposes the complete finding shape, ignore
directives skip named traits for a file, and a content-addressed cache avoids
repeat model requests.

## Sub-features

- `check-patterns` resolves explicit paths and globs and reports violations
  with exit code `1`.
- `check-compliant` distinguishes an assessed compliant file from an empty
  selection; both print `No findings.`
- `check-focused-trait` restricts the assessment to named traits.
- `check-json` prints structured findings with trait, source, location, and
  assessment.
- `check-ignore-directive` skips one named trait for a file while other traits
  still run.
- `check-chunking` splits a file at 200 lines and reports the later chunk's
  starting line.
- `check-cache` reuses conclusive answers; `--no-cache` and changed
  circumstances force new requests.

## How to get to it (user POV)

- Run `traits check [patterns]`.
- Add `--trait <name>` to focus one or more traits.
- Add `--json` for structured output.
- Add `--circumstances "<text>"` when the surrounding task changes
  applicability.
- Add `--no-cache` to bypass cached answers for one run.
- Add a leading `// traits-ignore-file <trait-name>: <reason>` comment to a
  source file.

## Driving it with control-traits

Preconditions:

- The doctor passes for this run ID.
- The fixture has `use-const` and `no-var` traits scoped to `*.ts`, plus
  `example.ts` (`let`), `good.ts` (`const`), `legacy.ts` (`var`),
  `ignored.ts` (a `use-const` ignore directive), and `big.ts` (201 lines with a
  `let` on line 201).
- The local mock provider answers violation for `let`/`var` and compliant
  otherwise. `exclude-gitignored #false` and a fixture-local `HOME` isolate the
  run.

- **Report a violation.** Run
  `./.opencode/skills/verify-traits/control-traits run check --run-id <id>`.
  `check/summary.json` has `check reports a violation with exit 1` and the
  output contains `violation use-const at <path>:1`.
- **Prove the compliant file was assessed.** The same run checks `a compliant
  file is still assessed`; the mock request count is at least `1` even though
  stdout is `No findings.`
- **Distinguish an empty selection.** The same run checks `an unmatched
  pattern makes no provider request`; `traits check missing.ts` exits `0` and
  makes zero requests.
- **Focus a trait.** The same run checks `a focused trait asks only one
  question`; `--trait no-var legacy.ts` reports `no-var` and not `use-const`.
- **Read JSON.** `check/findings.json` has one finding with `trait.name ==
  "use-const"`, `assessment.classification == "violation"`, and
  `source.location.startLine == 1`.
- **Respect an ignore directive.** The same run checks `the ignored file is
  still assessed for other traits`; `ignored.ts` makes a request for `no-var`
  but reports no finding because `use-const` is skipped.
- **Prove chunking.** The same run checks `the second chunk starts at line 201`
  and `the 201-line file needs two chunk requests`; `big.ts` reports the
  violation at line 201.
- **Prove the cache.** The same run warms the cache, checks `a repeat run makes
  no provider request`, then checks `--no-cache bypasses the cache`, `changing
  circumstances asks again`, and `repeating the same circumstances uses the
  cache`.
- **Proof.** Read `check/summary.json`, `check/findings.json`, and
  `check/check.json`.

## Gotchas

- `traits check` prints only violation and inconclusive findings. A compliant
  assessment is filtered out, so `No findings.` alone cannot distinguish
  "assessed compliant" from "no files selected." Check the mock request count,
  use `--json`, or focus a trait.
- An empty `patterns` list, an unmatched pattern, or a trait with no matching
  `paths` performs no assessment and makes no model request.
- Files are assessed in 200-line chunks. A finding's `startLine` is the first
  line of the chunk, not the exact offending line.
- Paths in findings are absolute. Assert the filename or suffix, not the
  fixture prefix.
- The cache key includes the provider identity, source text, trait question,
  and circumstances. A new mock port or a new trait body is a cache miss.
- The fixture cache lives under the fixture-local `HOME`; a new run ID starts
  cold.
- `traits-ignore-file` must be a leading `//` comment with exactly one trait
  name and a nonempty reason. It skips only that trait; other traits still run.
- The mock provider proves plumbing, not model judgment. It answers from the
  source text, so a finding is evidence that selection, chunking, transport,
  parsing, and reporting worked, not that a real model would agree.
