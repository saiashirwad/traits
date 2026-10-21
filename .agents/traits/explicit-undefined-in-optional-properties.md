---
name: explicit-undefined-in-optional-properties
purpose: Make optional-property assignment semantics explicit and convenient for callers passing possibly undefined values.
subject: Optional TypeScript property declarations.
requirement: "For TypeScript optional properties, explicitly include undefined
  in the property type by default: readonly copyable?: boolean | undefined, not
  readonly copyable?: boolean. Omit the undefined union only when the API
  intentionally disallows explicitly passing undefined while still permitting
  omission."
applicability: |-
  Applies when declaring optional properties.
  Exception: When every optional property under assessment intentionally permits omission but disallows explicitly passing undefined.
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
---
```diff
-readonly copyable?: boolean
+readonly copyable?: boolean | undefined
```
