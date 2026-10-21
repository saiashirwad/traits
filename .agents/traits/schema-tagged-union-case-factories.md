---
name: schema-tagged-union-case-factories
purpose: Keep tagged-union construction aligned with the schema's case definitions and validation behavior.
subject: Construction of Effect Schema tagged-union members.
requirement: 'Construct instances of Effect Schema tagged unions with their
  case-specific factories, not manually tagged object literals:
  Union.cases.Fail.make({ reason }), not { _tag: "Fail", reason }.'
applicability: |-
  Applies when constructing members of Effect Schema tagged unions.
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
---
```diff
-const outcome = { _tag: "Fail", reason }
+const outcome = Outcome.cases.Fail.make({ reason })
```
