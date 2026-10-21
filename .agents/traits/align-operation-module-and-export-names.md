---
name: align-operation-module-and-export-names
purpose: Make operations easy to locate and recognize without tracing aliases across modules.
subject: Operation-focused modules and their primary exports.
requirement: For operation-focused modules, keep the filename and primary export
  aligned with the operation's canonical name. When improving that name, rename
  the file and export together and update consumers to import it directly,
  rather than retaining the old source name behind an import alias.
applicability: |-
  Applies when naming or renaming an operation-focused module and its primary export.
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
-import { evaluate as check } from "./evaluate.ts"
+import { check } from "./check.ts"
```
