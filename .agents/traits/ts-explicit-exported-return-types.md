---
name: ts-explicit-exported-return-types
subject: Return types of exported TypeScript functions and public methods.
requirement: >-
  Annotate return types on exported functions and public methods. Let local
  helpers, inline callbacks, and trivial one-expression wrappers infer.
  Annotate overloads and generic APIs. The goal is a stable public contract and
  errors surfaced at the definition rather than at call sites.
applicability: |-
  Applies to module and public class boundaries.
  Exception: Trivial getters, immediately returned generic expressions, and
  projects that intentionally rely on inference for public APIs.
paths:
  - "*.ts"
  - "*.tsx"
  - "*.mts"
  - "*.cts"
purpose: Prevent accidental API widening and keep declaration output stable.
---
```diff
-export function parse(input: string) {
+export function parse(input: string): Config {
   return ConfigSchema.parse(JSON.parse(input))
 }
```

Sources: [typescript-eslint: explicit-module-boundary-types](https://typescript-eslint.io/rules/explicit-module-boundary-types/), [TypeScript Handbook: Functions](https://www.typescriptlang.org/docs/handbook/2/functions.html).
