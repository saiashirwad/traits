---
name: defects-for-internal-invariants
purpose: Keep public error channels focused on actionable failures while exposing internal invariant violations as defects.
subject: Library error channels for internal validation and encoding invariants.
requirement: "In library code, distinguish internal invariant violations from
  expected operational failures. Treat validation or encoding failures of
  internally constructed, already-typed data as defects when consumers cannot
  meaningfully recover: pipe(value, S.encodeEffect(schema), Effect.orDie),
  rather than exposing SchemaError in the public error channel. Preserve typed
  errors for invalid consumer input, configuration, network, storage, and other
  actionable operational failures; a handler not catching an error does not make
  it a defect."
applicability: |-
  Applies when designing library error channels for validation or encoding of internally constructed data.
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
---
```diff
-return pipe(internallyConstructed, S.encodeEffect(schema))
+return pipe(internallyConstructed, S.encodeEffect(schema), Effect.orDie)
```
