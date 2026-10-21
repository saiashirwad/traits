---
name: namespace-imports-for-local-modules
purpose: Make related local exports visibly belong to their module without repetitive aliases or crowded named imports.
subject: Imports of multiple related exports from local project modules.
requirement: For local project modules, prefer namespace imports and qualified
  member access when importing more than two exports or when multiple exports
  would otherwise have aliases sharing a module-name prefix. For example, use
  import * as Trait from "./Trait.ts" with Trait.parse, Trait.serialize,
  Trait.Trait, and Trait.TraitName.
applicability: |-
  Applies when importing more than two exports from a local project module or giving multiple local exports aliases with a shared module-name prefix.
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
-import { parse, serialize, Trait } from "./Trait.ts"
+import * as Trait from "./Trait.ts"
```
