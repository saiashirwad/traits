# Inspect the trait store

The store commands list the Markdown traits in `.agents/traits/`, print one
trait in canonical serialized form, and remove a trait by its canonical name.
The listing parses every file, so one malformed file fails the whole command.

## Sub-features

- `store-list` lists trait names and subjects in filename order.
- `store-show` prints one trait's canonical frontmatter and Markdown body.
- `store-remove` deletes one trait file by name.
- `store-parse-errors` surfaces malformed documents and filename/name
  mismatches instead of skipping them.

## How to get to it (user POV)

- Run `traits trait ls`.
- Run `traits trait show <name>`.
- Run `traits trait rm <name>`.
- Edit or add a file under `.agents/traits/<name>.md`, then run `traits trait
  ls` to validate the whole store.

## Driving it with control-traits

Preconditions:

- The doctor passes for this run ID.
- The fixture contains `.agents/traits/use-const.md` and
  `.agents/traits/no-var.md`.
- The fixture config names a provider and sets a dummy key; no model request is
  needed for store commands.

- **List the store.** Run
  `./.opencode/skills/verify-traits/control-traits run store --run-id <id>`.
  `store/summary.json` has `trait ls exits 0` and `trait ls is sorted by name`;
  `store/store.json` shows `no-var` before `use-const`.
- **Show one trait.** The same run checks `trait show prints canonical
  frontmatter` and `trait show includes the requirement`; stdout starts with
  `---` and contains `name: use-const`.
- **Show a missing trait.** The same run checks `showing a missing trait exits
  1`; stderr contains `Trait missing was not found`.
- **Reject a malformed file.** The same run writes a file without frontmatter
  and checks `a malformed trait fails the listing`; stderr contains
  `Unable to parse trait`.
- **Reject a filename mismatch.** The same run writes `wrong.md` whose
  frontmatter says `name: right-name` and checks `a filename mismatch fails the
  listing`; stderr contains `Trait filename must be right-name.md`.
- **Remove a trait.** The same run checks `trait rm removes the file` and
  `trait rm prints the removed name`; a second `trait ls` no longer lists
  `use-const`, and `trait show use-const` exits `1`.
- **Proof.** Read `store/summary.json` and `store/fixtures/use-const.md` and
  `store/fixtures/no-var.md`.

## Gotchas

- `traits trait ls` reads every `.md` file in the store. One malformed file or
  one filename/name mismatch fails the whole listing; it does not skip the bad
  entry.
- The filename must be `<name>.md`, and `name` must be lowercase kebab-case
  without a `trait-` prefix.
- `trait ls` separates the name and subject with an em dash (`—`), not a
  hyphen. Parse the name prefix, not whitespace.
- `trait rm` removes immediately; there is no confirmation prompt in the CLI.
- `trait show` prints the canonical serialization, which may differ in quoting
  or block style from a hand-written file.
