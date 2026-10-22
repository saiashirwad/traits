---
name: rust-async-resource-bounds
subject: Lock scope and task fan-out in Rust async code.
requirement: >-
  Do not hold a lock guard across an await; scope the lock so it drops before
  every await point. Bound fan-out over unbounded input with buffer_unordered,
  a semaphore, or an equivalent limit instead of spawning an unbounded set.
  Give every spawned task an owner that aborts or awaits it.
applicability: |-
  Exception: An async-aware lock around an I/O resource may need to span an
  await, and a fixed compile-time small fan-out may be unbounded in practice.
paths:
  - "*.rs"
purpose: Prevent deadlocks, stalled tasks, and memory spikes from unbounded concurrency.
---
```diff
-let guard = mutex.lock().await;
-work().await;
+let value = { mutex.lock().await.clone() };
+work(value).await;
```

Sources: [tokio::sync::Mutex](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html), [futures: buffer_unordered](https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.buffer_unordered), [tokio::task](https://docs.rs/tokio/latest/tokio/task/index.html).
