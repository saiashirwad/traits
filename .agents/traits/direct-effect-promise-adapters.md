---
name: direct-effect-promise-adapters
purpose: Make asynchronous boundaries explicit without wrappers that add indirection but no behavior.
subject: Adapters from Promise-returning operations to Effect.
requirement: "Use `Effect.promise` directly at Promise boundaries instead of
  introducing generic wrapper helpers: `Effect.promise(() => client.run())`, not
  `fromPromise(() => client.run())` where `fromPromise` merely wraps
  `Effect.promise`."
applicability: |-
  Applies when adapting Promise-returning operations to Effect.
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
---
```diff
-const result = fromPromise(() => client.run()) // Helper only wraps Effect.promise.
+const result = Effect.promise(() => client.run())
```
