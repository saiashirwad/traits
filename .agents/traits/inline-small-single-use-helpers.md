---
name: inline-small-single-use-helpers
purpose: Keep local behavior easy to follow without unnecessary helper indirection.
subject: Small single-use helpers, including React atom logic.
requirement: Prefer inlining small single-use helper logic into the operation
  that consumes it instead of introducing a separately named helper, such as
  placing theme initialization directly inside its atom definition. Extract a
  helper when reused, when naming materially improves readability, or when it
  defines a meaningful domain/API boundary. In React, inline into the relevant
  atom or Effect operation, not into component render or event handlers.
applicability: |-
  Exception: When helper extraction materially improves readability through naming or defines a meaningful domain or API boundary.
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
-const getName = (item) => item.name // Trivial single-use helper.
-const names = Array.map(items, getName)
+const names = Array.map(items, (item) => item.name)
```
