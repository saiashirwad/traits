---
name: construct-values-inside-pipelines
purpose: Let readers follow transformations and final construction in execution order.
subject: Final value construction in transformation pipelines.
requirement: "Keep final value construction inside a transformation pipeline
  rather than wrapping the pipeline in a constructor call: use pipe(values,
  transform, (items) => new Set(items)), not new Set(pipe(values, transform)).
  This also applies to other constructors when the construction is the final
  transformation."
applicability: |-
  Applies when constructing a final value from a transformation pipeline.
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
---
```diff
-const names = new Set(pipe(items, Array.map((item) => item.name)))
+const names = pipe(items, Array.map((item) => item.name), (values) => new Set(values))
```
