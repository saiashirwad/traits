---
name: rust-reserve-known-capacity
subject: Vec, String, and map construction when the final size is known.
requirement: >-
  When the final element count is known or tightly bounded before a fill loop,
  reserve once with Vec::with_capacity, String::with_capacity, reserve, or
  collect from an exact-size iterator instead of growing element by element.
  Use Vec::extend to append an existing sequence. Do not reserve speculative
  capacities; an estimate derived from the input count is fine, but reject
  attacker-controlled lengths that can waste memory or enable denial of
  service.
applicability: |-
  Exception: Small or unknown sizes, vec![0; n], and iterators with unreliable
  size hints.
paths:
  - "*.rs"
purpose: Avoid repeated reallocation and copying when the required size is already known.
---
```diff
-let mut out = Vec::new();
-for item in items {
-    out.push(transform(item));
-}
+let mut out = Vec::with_capacity(items.len());
+for item in items {
+    out.push(transform(item));
+}
```

Sources: [Rust Performance Book: Heap Allocations](https://nnethercote.github.io/perf-book/heap-allocations.html), [Clippy: vec_init_then_push](https://rust-lang.github.io/rust-clippy/master/index.html#vec_init_then_push), [std::vec::Vec::with_capacity](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.with_capacity).
