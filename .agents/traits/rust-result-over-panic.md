---
name: rust-result-over-panic
subject: Fallible operations in Rust library and application code.
requirement: >-
  Return Result for recoverable failures and propagate with the question-mark
  operator. Reserve panic!, unwrap, and expect for proven invariants, tests,
  examples, build scripts, and the top-level main function. When expect is
  justified, its message states the invariant rather than restating the call.
applicability: |-
  Applies to fallible operations outside tests, examples, and build scripts.
  Exception: A panic may enforce a documented programmer contract when the
  caller cannot recover and construction has already checked the condition.
  unwrap and expect are also acceptable for lock poisoning, infallible
  formatting or String writes, and a value proven in the same function.
paths:
  - "*.rs"
purpose: Keep recoverable failures in the type system so callers can choose how to respond.
---
```diff
-let n: u32 = raw.parse().unwrap();
+let n: u32 = raw.parse()?;
```

Sources: [The Rust Book: Recoverable Errors with Result](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html), [The Rust Book: To Panic or Not to Panic](https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html), [Effective Rust: Panics](https://www.lurklurk.org/effective-rust/panic.html), [Clippy: unwrap_used](https://rust-lang.github.io/rust-clippy/master/index.html#unwrap_used).
