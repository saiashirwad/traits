---
name: go-measure-before-optimizing
subject: Performance-motivated Go changes.
requirement: >-
  A non-obvious performance optimization must reference a benchmark or profile
  covering the affected path, or be left as the clearer form. A benchmark used
  as evidence reports allocations with b.ReportAllocs. Do not add complexity
  based on intuition alone.
applicability: |-
  Exception: Standard allocation-avoidance idioms with documented cost, fixed
  quadratic or unbounded work, correctness and readability fixes, and cold
  startup.
guidance: b.Loop requires Go 1.24 or newer; use for i := 0; i < b.N; i++ on older versions.
paths:
  - "*.go"
purpose: Measured improvements are the only defensible reason to add performance complexity.
---
```diff
-// "faster" rewrite without evidence
+// BenchmarkRender reports ns/op and allocs/op; compare with benchstat.
+func BenchmarkRender(b *testing.B) {
+    b.ReportAllocs()
+    for b.Loop() {
+        Render(input)
+    }
+}
```

Sources: [testing: Benchmarks](https://pkg.go.dev/testing#hdr-Benchmarks), [benchstat](https://pkg.go.dev/golang.org/x/perf/cmd/benchstat), [Go Blog: Profiling Go Programs](https://go.dev/blog/profiling-go-programs).
