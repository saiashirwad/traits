---
name: inline-single-use-schema-and-decision-definitions
purpose: Keep lightweight definitions near their use and avoid names that add navigation without a meaningful boundary.
subject: Single-use schemas, transcoders, and lightweight Decision.make definitions.
requirement: "Prefer inlining single-use schemas, schema transcoder definitions,
  and lightweight Decision.make definitions at their call sites instead of
  adding separate constants and names: DecisionModel.decide(Decision.make({
  input: S.Struct({ text: S.String }), decisions: { ... } }), options). Extract
  definitions when reused, when needed for readability, or when profiling shows
  meaningful construction overhead."
applicability: |-
  Applies when defining single-use schemas, transcoders, or lightweight Decision.make values.
  Exception: When extracting a schema, transcoder, or Decision.make definition is needed for readability or justified by profiled construction overhead.
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
---
```diff
-const inputSchema = S.Struct({ text: S.String }) // Used once, no meaningful boundary.
-const input = yield* pipe(value, S.decodeUnknownEffect(inputSchema))
+const input = yield* pipe(value, S.decodeUnknownEffect(S.Struct({ text: S.String })))
```
