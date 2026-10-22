---
name: rust-minimal-public-surface
subject: Public visibility and documentation of Rust API items.
requirement: >-
  Default items and fields to private. Widen to pub(crate) or pub(super) for
  internal use, and pub only for a genuine external API, integration-test
  contract, or FFI boundary. Keep fields that carry invariants private.
applicability: |-
  Applies to library crates and reusable modules.
  Exception: Binary crates, generated bindings, facade re-exports, and plain
  data-transfer structs whose public fields carry no invariant and are intended
  for direct construction or serialization. Do not flag a public field merely
  for being public; the rule targets fields whose mutation could break an
  invariant the type is meant to protect.
paths:
  - "*.rs"
purpose: Public API is a semver commitment; a small surface preserves invariants and refactoring freedom.
---
```diff
-pub struct Parser { pub state: Vec<Token> }
+pub struct Parser { state: Vec<Token> }
```

Sources: [Rust API Guidelines: C-STRUCT-PRIVATE](https://rust-lang.github.io/api-guidelines/future-proofing.html#c-struct-private), [Effective Rust: Visibility](https://www.lurklurk.org/effective-rust/visibility.html).
