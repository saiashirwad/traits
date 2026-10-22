---
name: rust-typed-error-enums
subject: Public error types in Rust libraries.
requirement: >-
  Public error types implement Debug, Display, and std::error::Error. Represent
  distinct failure cases as variants and preserve underlying failures through a
  From impl, thiserror's #[from], source(), or explicit wrapping; do not flatten
  causes to String. Use anyhow or Box<dyn Error> only at application edges,
  binaries, or prototypes, not in a library's public signature.
applicability: |-
  Applies to public and crate-public fallible APIs.
  Exception: Opaque error types that deliberately hide implementation details,
  and no_std code using core::error::Error.
paths:
  - "*.rs"
purpose: Callers need structured failures they can match on and diagnosticians need the original cause.
---
```diff
-fn load(path: &Path) -> Result<Config, String> {
-    std::fs::read_to_string(path).map_err(|e| e.to_string())
-}
+fn load(path: &Path) -> Result<Config, LoadError> {
+    let text = std::fs::read_to_string(path)?; // LoadError::Io(#[from] io::Error)
+    parse(&text) // LoadError::Parse(#[from] ParseError)
+}
```

Sources: [Rust API Guidelines: C-GOOD-ERR](https://rust-lang.github.io/api-guidelines/interoperability.html#error-types-are-meaningful-and-well-behaved-c-good-err), [Effective Rust: Errors](https://www.lurklurk.org/effective-rust/errors.html), [The Rust Book: Recoverable Errors](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html).
