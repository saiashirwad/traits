---
name: rust-newtype-domain-values
subject: Raw primitives used for domain identifiers, units, money, and validated scalars.
requirement: >-
  When two values share a representation but not a meaning, wrap them in a
  newtype with a private field and a validating constructor. Apply this to
  domain identifiers, units, money, and validated scalars. Keep the newtype
  zero-cost with accessors and From or TryFrom where appropriate.
applicability: |-
  Exception: Local variables inside one function, FFI and repr(transparent)
  boundaries, serialization DTOs, trivial wrappers with no invariant, and
  allocation-sensitive hot paths where the wrapper would not be optimized away.
paths:
  - "*.rs"
purpose: Make argument transposition and unit mix-ups compile errors instead of production bugs.
---
```diff
-fn transfer(from: String, to: String, amount: u64) { ... }
+fn transfer(from: AccountId, to: AccountId, amount: Cents) { ... }
```

Sources: [Rust API Guidelines: C-NEWTYPE and C-CUSTOM-TYPE](https://rust-lang.github.io/api-guidelines/type-safety.html#c-newtype), [Effective Rust: Newtypes](https://www.lurklurk.org/effective-rust/newtype.html).
