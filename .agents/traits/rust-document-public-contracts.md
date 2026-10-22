---
name: rust-document-public-contracts
subject: Documentation of public Rust API items.
requirement: >-
  Public functions and methods that can fail, panic, or require unsafe
  preconditions document those conditions in # Errors, # Panics, or # Safety.
  Public types, traits, functions, methods, and modules have a /// summary.
  Do not assess public fields or enum variants separately; their documentation
  belongs to the parent item.
applicability: |-
  Exception: pub use re-exports, doc(hidden) items, generated items, and trait
  impl methods that inherit documentation.
guidance: Document public fields and enum variants when their meaning or invariant is not obvious from the name.
paths:
  - "*.rs"
purpose: Rustdoc is the API contract; callers need failure modes, panic conditions, and safety obligations.
---
```diff
+/// Parses one expression.
+/// # Errors
+/// Returns `ParseError` if the input is empty or malformed.
+pub fn parse(input: &str) -> Result<Ast, ParseError> { ... }
```

Sources: [Rust API Guidelines: C-FAILURE](https://rust-lang.github.io/api-guidelines/documentation.html#function-docs-include-error-panic-and-safety-considerations-c-failure), [The rustdoc Book](https://doc.rust-lang.org/rustdoc/).
