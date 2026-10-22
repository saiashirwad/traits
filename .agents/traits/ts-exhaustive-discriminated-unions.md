---
name: ts-exhaustive-discriminated-unions
subject: Switch and if chains over closed TypeScript discriminated unions.
requirement: >-
  Handle every member of a closed discriminated union explicitly. End with a
  terminal that proves exhaustiveness: assign the remaining value to never,
  call an assertNever helper, use Match.exhaustive, or throw. A documented
  default is acceptable only for an open union.
applicability: |-
  Exception: Open unions such as vendor events or extension points may use a
  documented fallback.
paths:
  - "*.ts"
  - "*.tsx"
  - "*.mts"
  - "*.cts"
purpose: Adding a variant should be a compile error, not a runtime fallthrough.
---
```diff
 switch (state.kind) {
   case "idle": return null
   case "loading": return spinner
+  case "ready": return view(state.data)
+  default: {
+    const unreachable: never = state
+    throw new Error(`unhandled state: ${unreachable}`)
+  }
 }
```

Sources: [TypeScript Handbook: Exhaustiveness Checking](https://www.typescriptlang.org/docs/handbook/2/narrowing.html#exhaustiveness-checking), [typescript-eslint: switch-exhaustiveness-check](https://typescript-eslint.io/rules/switch-exhaustiveness-check/).
