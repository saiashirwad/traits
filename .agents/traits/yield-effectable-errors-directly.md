---
name: yield-effectable-errors-directly
purpose: Express effectable failures directly without redundant failure wrappers.
subject: Returning and yielding error values that are themselves effectable.
requirement: "When an Effect error value is itself effectable, return or yield
  it directly instead of wrapping it with Effect.fail: return error, not return
  Effect.fail(error). Keep Effect.fail for error values that are not
  effectable."
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
---
```diff
-return yield* Effect.fail(MissingDocument.make({ path }))
+return yield* MissingDocument.make({ path })
```
