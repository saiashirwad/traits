---
name: tagged-union-case-guards
purpose: Keep case detection aligned with union definitions and preserve useful type narrowing without manual tag checks.
subject: Case checks and filtering for tagged unions with built-in guards.
requirement: 'Use tagged unions’ built-in case guards when testing or filtering
  cases rather than manually comparing the tag:
  Array.filter(TraitExpression.guards.Fail), not Array.filter((value) =>
  value._tag === "Fail").'
applicability: |-
  Applies when testing or filtering cases of tagged unions with built-in case guards.
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
---
```diff
-const failures = Array.filter(results, (result) => result._tag === "Fail")
+const failures = Array.filter(results, Outcome.guards.Fail)
```
