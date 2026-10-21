---
name: flat-transformation-pipelines
purpose: Make transformation order readable without mentally unwinding nested calls.
subject: Successive transformations and Effect.fn transformation arguments.
requirement: Use `.pipe(...)` to keep successive transformations flat and
  readable in execution order, rather than nesting transformation calls around
  an expression. For example, prefer `Effect.gen(...).pipe(Layer.unwrap,
  Layer.provide(dependencies))` over
  `Layer.unwrap(Effect.gen(...)).pipe(Layer.provide(dependencies))`. Keep
  successive transformations in the same pipe where practical. For `Effect.fn`
  and `Effect.fnUntraced`, use their transformation arguments as the equivalent
  flat composition style.
applicability: |-
  Applies when composing successive transformations or Effect.fn transformation arguments.
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
---
```diff
-const layer = Layer.unwrap(effect).pipe(Layer.provide(dependencies))
+const layer = effect.pipe(Layer.unwrap, Layer.provide(dependencies))
```
