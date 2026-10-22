---
name: rust-async-only-when-awaiting
subject: Async functions that never await.
requirement: >-
  Mark a function async only when it actually awaits or must satisfy an async
  trait or API contract. Do not make synchronous CPU or filesystem helpers
  async merely for uniformity. Remove async when the body has no await and the
  signature is under your control.
applicability: |-
  Applies to Rust functions you can change.
  Exception: Trait implementations with fixed async signatures, API stability,
  and functions that intentionally return a future without awaiting internally.
paths:
  - "*.rs"
purpose: Avoid infecting callers with executor requirements and hiding blocking work behind an async signature.
---
```diff
-async fn load(path: &Path) -> Vec<u8> {
-    std::fs::read(path).unwrap()
-}
+fn checksum(data: &[u8]) -> u64 {
+    hash(data)
+}
```

Sources: [Clippy: unused_async](https://rust-lang.github.io/rust-clippy/master/index.html#unused_async), [Alice Ryhl: Async, What Is Blocking?](https://ryhl.io/blog/async-what-is-blocking/), [tokio::task::spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html).
