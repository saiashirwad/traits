---
name: rust-iterate-not-index
subject: Index-based loops over Rust collections.
requirement: >-
  Iterate collections directly, using iter, iter_mut, enumerate, zip, windows,
  chunks, retain, drain, or extend instead of for i in 0..v.len() plus
  indexing. Use an explicit index only when the algorithm needs multiple
  indices, accesses parallel collections, or mutates non-locally.
applicability: |-
  Exception: Numeric ranges that are not collections, and cases where an
  explicit index reads more clearly than a complex iterator chain.
paths:
  - "*.rs"
purpose: Express intent directly, avoid bounds-check noise, and eliminate off-by-one bugs.
---
```diff
-for i in 0..values.len() {
-    total += values[i];
-}
+for value in &values {
+    total += *value;
+}
```

Sources: [Clippy: needless_range_loop](https://rust-lang.github.io/rust-clippy/master/index.html#needless_range_loop), [Effective Rust: Iterators](https://www.lurklurk.org/effective-rust/iterators.html), [Rust Performance Book: Iterators](https://nnethercote.github.io/perf-book/iterators.html).
