---
name: tagged-enum-runtime-factories
purpose: Provide consistent runtime construction and matching support alongside tagged-enum type declarations.
subject: Effect Data.TaggedEnum type aliases.
requirement: "Every Effect Data.TaggedEnum type alias must be paired with a
  same-named runtime Data.taggedEnum factory, exposing both the type and value
  under one name: export type Outcome = Data.TaggedEnum<{ Pass: {}; Fail: {
  reason: string } }>; export const Outcome = Data.taggedEnum<Outcome>(). This
  applies to Data.TaggedEnum declarations, independently of Schema.TaggedUnion
  construction rules."
applicability: |-
  Applies when declaring an Effect Data.TaggedEnum type alias.
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
---
```diff
-type Outcome = Data.TaggedEnum<{ Pass: {}; Fail: { reason: string } }>
+type Outcome = Data.TaggedEnum<{ Pass: {}; Fail: { reason: string } }>
+const Outcome = Data.taggedEnum<Outcome>()
```
