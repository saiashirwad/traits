---
name: go-preallocate-known-collections
subject: Slice and map capacity when the final size is known.
requirement: >-
  Before a fill loop with a known or tightly bounded element count, use
  make([]T, 0, n) for append or make([]T, n) for index assignment; use
  slices.Grow when growing an existing slice; use make(map[K]V, n) for maps
  with a known approximate size. Do not preallocate speculative or
  attacker-controlled huge capacities.
applicability: |-
  Exception: Small or unknown sizes, map capacity is only a hint, a filtering
  loop where the input length is an upper bound that may over-allocate, and
  clarity may favor append without an explicit capacity for very small n.
paths:
  - "*.go"
purpose: Avoid repeated growth, allocation, and copying when the required size is already known.
---
```diff
-var out []Item
-for _, item := range items {
-    out = append(out, transform(item))
-}
+out := make([]Item, 0, len(items))
+for _, item := range items {
+    out = append(out, transform(item))
+}
```

Sources: [slices.Grow](https://pkg.go.dev/slices#Grow), [Go GC Guide: Eliminating Heap Allocations](https://go.dev/doc/gc-guide#Eliminating_heap_allocations), [Go Blog: Profiling Go Programs](https://go.dev/blog/profiling-go-programs).
