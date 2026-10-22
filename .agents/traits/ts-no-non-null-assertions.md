---
name: ts-no-non-null-assertions
subject: Postfix non-null assertions in TypeScript.
requirement: >-
  Establish non-nullness with a real check, guard clause, or invariant helper
  instead of the postfix exclamation assertion. Use the assertion only when a
  preceding line has proven presence in a way TypeScript cannot see, and add a
  comment explaining the invariant.
applicability: |-
  Exception: Tests and generated code, and a documented invariant immediately
  above the assertion.
paths:
  - "*.ts"
  - "*.tsx"
  - "*.mts"
  - "*.cts"
purpose: The assertion verifies nothing and converts a compile-time concern into a production TypeError.
---
```diff
-return map.get(key)!.name
+const value = map.get(key)
+if (value === undefined) {
+  throw new Error(`missing value for ${key}`)
+}
+return value.name
```

Sources: [typescript-eslint: no-non-null-assertion](https://typescript-eslint.io/rules/no-non-null-assertion/), [Google TypeScript Style Guide: Non-nullability Assertions](https://google.github.io/styleguide/tsguide.html).
