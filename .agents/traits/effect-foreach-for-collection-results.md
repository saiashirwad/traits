---
name: effect-foreach-for-collection-results
purpose: Express effectful traversal and result collection together with visible ordering and concurrency semantics.
subject: Effectful collection traversals that collect results.
requirement: When traversing a collection to perform Effects and collect
  results, prefer Effect.forEach over a for loop that pushes into a mutable
  accumulator array. Preserve intended ordering, filtering, and concurrency
  semantics.
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
---
```diff
-const results = []
-for (const item of items) results.push(yield* processItem(item))
+const results = yield* Effect.forEach(items, processItem)
```
