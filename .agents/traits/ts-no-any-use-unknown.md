---
name: ts-no-any-use-unknown
subject: TypeScript annotations that claim any.
requirement: >-
  Annotate uncertain values as unknown and narrow before use. Introduce any
  only behind a scoped, commented escape hatch for untyped third-party
  declarations or test mocks; do not use any as a convenience in application
  code.
applicability: |-
  Exception: Declaration shims, generated code, and test mocks with a comment
  explaining why the type is unknowable.
paths:
  - "*.ts"
  - "*.tsx"
  - "*.mts"
  - "*.cts"
purpose: any silently disables checking and propagates through every call site; unknown forces a decision at the boundary.
---
```diff
-function load(value: any) {
-  return value.id
-}
+function load(value: unknown) {
+  if (!isEntity(value)) throw new TypeError("expected entity")
+  return value.id
+}
```

Sources: [typescript-eslint: no-explicit-any](https://typescript-eslint.io/rules/no-explicit-any/), [Google TypeScript Style Guide](https://google.github.io/styleguide/tsguide.html), [TypeScript: strict](https://www.typescriptlang.org/tsconfig/strict.html).
