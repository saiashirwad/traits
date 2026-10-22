---
name: rust-measure-before-optimizing
subject: Performance-motivated Rust changes and speculative micro-optimizations.
requirement: >-
  A performance-motivated change must be justified by a measurement on a
  realistic workload: a benchmark, profiler, or trace, run on an optimized
  build. A non-obvious optimization that remains in the code carries a comment
  naming the measurement or profile that motivated it. Do not add speculative
  micro-optimizations such as inline(always), get_unchecked, SmallVec, custom
  hashers, or manual loop rewrites without evidence that the path is hot.
applicability: |-
  Applies when performance is the stated reason for a change.
  Exception: Fixing an obviously quadratic algorithm or an unbounded allocation
  is justified by inspection; cold startup and one-shot code are exempt;
  correctness and readability fixes need no benchmark.
paths:
  - "*.rs"
purpose: Spend complexity only where measured cost justifies it and avoid cargo-cult performance rules.
---
```diff
-#[inline(always)]
-fn parse(input: &str) -> Ast { ... }
+fn parse(input: &str) -> Ast { ... }
+// Benchmark `parse` first; add #[inline] only if the profile shows call overhead.
```

Sources: [Rust Performance Book: Benchmarking](https://nnethercote.github.io/perf-book/benchmarking.html), [Rust Performance Book: Build Configuration](https://nnethercote.github.io/perf-book/build-configuration.html), [Clippy lint groups: perf](https://doc.rust-lang.org/clippy/lints.html#perf).
