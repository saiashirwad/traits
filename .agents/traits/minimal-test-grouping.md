---
name: minimal-test-grouping
purpose: Keep test navigation focused on meaningful behavior rather than redundant nesting.
subject: Test-file organization and describe blocks.
requirement: "Keep test files minimally structured: write tests at the top level
  and omit outer describe blocks that merely repeat the filename or subject. Add
  named describe blocks only when they organize meaningful behavior groups."
applicability: |-
  Applies when writing or organizing test files and describe blocks.
  Exception: When every describe block under assessment organizes a meaningful behavior group rather than merely repeating the filename or subject.
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
  - "*.js"
  - "*.mjs"
  - "*.cjs"
  - "*.jsx"
---
```diff
-describe("parse", () => { it("preserves the body", testBody) })
+it("preserves the body", testBody)
```
