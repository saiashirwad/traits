---
name: rust-no-blocking-in-async
subject: Blocking work inside Rust async functions and tasks.
requirement: >-
  In async functions and tasks, do not perform blocking I/O, thread::sleep, or
  long CPU work without yielding. Offload blocking work with spawn_blocking or
  a dedicated pool, and use async I/O where one exists.
applicability: |-
  Exception: block_in_place on a multi-thread runtime, very short CPU sections
  that are measured and documented, and runtime APIs that provide their own
  blocking bridge.
paths:
  - "*.rs"
purpose: Keep executor threads responsive and prevent starvation from tasks that never yield.
---
```diff
-async fn load(path: &Path) -> Vec<u8> {
-    std::fs::read(path).unwrap()
-}
+async fn load(path: &Path) -> std::io::Result<Vec<u8>> {
+    tokio::fs::read(path).await
+}
```

Sources: [Alice Ryhl: Async, What Is Blocking?](https://ryhl.io/blog/async-what-is-blocking/), [tokio::task::spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html), [tokio::fs](https://docs.rs/tokio/latest/tokio/fs/index.html).
