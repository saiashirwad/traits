---
name: ts-measure-before-optimizing
subject: Performance-motivated TypeScript and JavaScript changes.
requirement: >-
  A non-obvious performance optimization must carry a comment naming the
  measurement, tool, workload, and observed before-and-after effect that
  motivated it. Prefer the clearer form when no such evidence exists. Do not
  restructure clear code for speculative micro-optimization.
applicability: |-
  Exception: Fixing an obviously unbounded or quadratic operation; correctness
  and readability changes; cold startup and small scripts.
paths:
  - "*.ts"
  - "*.tsx"
  - "*.mts"
  - "*.cts"
  - "*.js"
  - "*.mjs"
  - "*.cjs"
  - "*.jsx"
purpose: Optimizing unmeasured code adds complexity without improving user-visible latency.
---
```diff
-// Rewrite every .map().filter() "for speed"
+// Profiled: parser is 40% of CPU; benchmark went 120ms -> 45ms.
```

Sources: [MDN: Performance API](https://developer.mozilla.org/en-US/docs/Web/API/Performance_API), [Node.js: Profiling](https://nodejs.org/learn/getting-started/profiling), [V8: Real-World Performance](https://v8.dev/blog/real-world-performance).
