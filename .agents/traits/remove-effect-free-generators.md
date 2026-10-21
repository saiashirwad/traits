---
name: remove-effect-free-generators
purpose: Make genuine effect sequencing stand out by removing generator scaffolding that adds no sequencing behavior.
subject: Effect.gen and Effect.fn generator wrappers and Layer construction.
requirement: "Remove unnecessary Effect.gen and Effect.fn generator wrappers:
  return an existing value, Effect, or Layer directly when the generator merely
  returns it or only yields a single existing Effect pipeline. A lone yield or
  return yield does not justify a generator. Retain generators that sequence
  multiple effectful steps. This rule concerns whether a generator is necessary;
  Promise adapters, parameterless operation exports, tracing, and
  transformation-chain layout are separate concerns."
applicability: |-
  Applies when introducing or reviewing Effect.gen wrappers and Layer construction.
  Exception: When every Effect.gen wrapper under assessment sequences effectful work rather than merely returning an existing value, Effect, or Layer.
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
---
```diff
-const load = Effect.gen(function* () { return yield* read })
+const load = read
```
