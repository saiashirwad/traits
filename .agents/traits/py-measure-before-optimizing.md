---
name: py-measure-before-optimizing
subject: Performance-motivated Python changes.
requirement: >-
  Identify the hot path with a profiler, then verify a change with the same
  benchmark before and after: cProfile or pstats for cumulative and self time,
  timeit or pyperf for repeatable timing. A non-obvious optimization carries a
  comment naming the tool and workload that motivated it. Do not add
  speculative micro-optimizations, __slots__, caching, or concurrency without
  evidence.
applicability: |-
  Exception: Fixing an obviously quadratic algorithm or unbounded memory use is
  justified by inspection; correctness and readability changes need no
  benchmark; cold startup and one-shot scripts.
paths:
  - "*.py"
purpose: Python performance intuition is unreliable, and profilers separate algorithmic cost from local cost.
---
```diff
-# "list comprehension is faster" rewrite without evidence
+# Profile first: cProfile -s cumulative shows parse_time as the hot path.
+# timeit before/after on the same workload; keep the change only if it wins.
```

Sources: [timeit](https://docs.python.org/3/library/timeit.html), [cProfile and pstats](https://docs.python.org/3/library/profile.html), [pyperf](https://pyperf.readthedocs.io/en/latest/).
