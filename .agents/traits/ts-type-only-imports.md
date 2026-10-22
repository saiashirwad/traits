---
name: ts-type-only-imports
subject: Imports used only in type position in TypeScript.
requirement: >-
  Import types with import type or inline type modifiers so they are erased and
  cannot create runtime cycles. Keep normal imports for values used at runtime,
  including classes, const objects, and enums.
applicability: |-
  Exception: Namespace imports where the runtime value is used, import-equals
  declarations, and generated code.
guidance: When verbatimModuleSyntax is enabled, follow its exact import form.
paths:
  - "*.ts"
  - "*.tsx"
  - "*.mts"
  - "*.cts"
purpose: Explicit type imports make elision predictable and prevent dead modules from surviving bundling.
---
```diff
-import { User } from "./user"
+import type { User } from "./user"
```

Sources: [TypeScript 3.8: Type-Only Imports](https://www.typescriptlang.org/docs/handbook/release-notes/typescript-3-8.html), [TypeScript: verbatimModuleSyntax](https://www.typescriptlang.org/tsconfig/verbatimModuleSyntax.html), [typescript-eslint: consistent-type-imports](https://typescript-eslint.io/rules/consistent-type-imports/).
