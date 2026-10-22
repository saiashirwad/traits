---
name: types-over-boolean-flags
subject: Parameters and state combinations expressed with primitives or booleans.
requirement: >-
  Replace a boolean parameter that selects behavior with a named enum, union,
  or separate functions. Group fields that must be present together into a
  required record, optionally wrapped in a sum type for presence or absence.
  Give identifiers and units distinct types when mixing them would be a bug and
  the language supports it.
applicability: |-
  Exception: Independent toggles such as disabled or readOnly; boolean
  parameters at keyword-only call sites; wire and database DTOs; FFI layouts;
  public API compatibility; and allocation-sensitive hot paths.
purpose: Types make invalid combinations unrepresentable and call sites read as intent rather than positional truth values.
---
```diff
-sendEmail(user, true, false)
+sendEmail(user, { mode: "immediate", includeReceipt: false })
 
-const address = { street: null, city: null, country: null }
+const address: Address | null = null
```

Sources: [Rust API Guidelines: Type Safety](https://rust-lang.github.io/api-guidelines/type-safety.html), [Refactoring: Remove Flag Argument](https://refactoring.com/catalog/removeFlagArgument.html), [TypeScript Handbook: Discriminated Unions](https://www.typescriptlang.org/docs/handbook/2/narrowing.html#discriminated-unions).
