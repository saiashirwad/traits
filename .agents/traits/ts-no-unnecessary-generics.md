---
name: ts-no-unnecessary-generics
subject: Type parameters in TypeScript that do not relate multiple types.
requirement: >-
  A generic type parameter must relate two or more types, such as an input to
  an output or a constraint to a return, or provide meaningful type safety for
  a factory or registry. If it appears once in the signature and nowhere in the
  body, use a concrete type or unknown.
applicability: |-
  Exception: Factory and registry APIs that thread a type token, generics that
  materially improve caller error messages, and public API stability.
guidance: Use const type parameters when literal inference is intended.
paths:
  - "*.ts"
  - "*.tsx"
  - "*.mts"
  - "*.cts"
purpose: Single-use generics add inference noise and let callers choose a type the implementation ignores.
---
```diff
-function id<T>(value: T): string {
+function id(value: unknown): string {
   return String(value)
 }
```

Sources: [typescript-eslint: no-unnecessary-type-parameters](https://typescript-eslint.io/rules/no-unnecessary-type-parameters/), [TypeScript Handbook: Generics](https://www.typescriptlang.org/docs/handbook/2/generics.html).
