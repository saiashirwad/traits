---
name: ts-readonly-at-boundaries
subject: Mutation of caller-owned data and readonly TypeScript types.
requirement: >-
  Accept readonly T[] or Readonly<T> in functions that do not mutate their
  inputs, and do not mutate caller-owned data. Copy before sorting or
  modifying. Treat readonly as a compile-time signal, not runtime immutability.
applicability: |-
  Exception: Builders, accumulators, third-party mutable APIs, and
  framework-owned state.
guidance: Mark never-reassigned properties readonly at API boundaries.
paths:
  - "*.ts"
  - "*.tsx"
  - "*.mts"
  - "*.cts"
purpose: Signal intent, prevent accidental aliased mutation, and make safe sharing explicit.
---
```diff
-function total(items: Item[]): number {
-  items.sort(compare)
-  ...
-}
+function total(items: readonly Item[]): number {
+  const sorted = [...items].sort(compare)
+  return sorted.reduce((sum, item) => sum + item.price, 0)
+}
```

Sources: [TypeScript Handbook: readonly Properties](https://www.typescriptlang.org/docs/handbook/2/objects.html#readonly-properties), [Google TypeScript Style Guide: Use readonly](https://google.github.io/styleguide/tsguide.html), [typescript-eslint: prefer-readonly-parameter-types](https://typescript-eslint.io/rules/prefer-readonly-parameter-types/).
