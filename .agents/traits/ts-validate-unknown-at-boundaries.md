---
name: ts-validate-unknown-at-boundaries
subject: Type assertions applied to externally sourced TypeScript data.
requirement: >-
  Keep externally sourced data, including JSON.parse, fetch, environment,
  localStorage, IPC, and URL parameters, as unknown and narrow it with a schema
  or type guard before it enters typed domain code. Never use as DomainType on
  unvalidated input. Type assertions are only for values whose runtime shape is
  already proven.
applicability: |-
  Exception: Internal round-trips of already-validated data, test fixtures, and
  an assertion applied after a schema or guard has validated the value.
paths:
  - "*.ts"
  - "*.tsx"
  - "*.mts"
  - "*.cts"
purpose: A type assertion is a compile-time claim about runtime bytes and fails exactly where data is untrusted.
---
```diff
-const config = JSON.parse(text) as Config
+const config = ConfigSchema.parse(JSON.parse(text))
```

Sources: [typescript-eslint: no-unsafe-type-assertion](https://typescript-eslint.io/rules/no-unsafe-type-assertion/), [Google TypeScript Style Guide](https://google.github.io/styleguide/tsguide.html), [TypeScript Handbook: Narrowing](https://www.typescriptlang.org/docs/handbook/2/narrowing.html).
