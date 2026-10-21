---
name: filter-or-fail-for-effect-validation
purpose: Keep predicate validation and its failure path visible in the Effect pipeline.
subject: Validation of Effect results solely to fail on a predicate and otherwise continue.
requirement: When validating an Effect result only to fail on a predicate and
  otherwise continue, prefer Effect.filterOrFail in the effect’s pipe over
  yielding into a temporary variable and branching to raise an error. Preserve
  the existing error type and diagnostic details.
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
---
```diff
-const value = yield* load
-if (!isValid(value)) return yield* InvalidValue.make({ value })
+const value = yield* load.pipe(Effect.filterOrFail(isValid, (value) => InvalidValue.make({ value })))
```
