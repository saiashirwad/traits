---
name: ts-error-cause
subject: Thrown and rejected values in TypeScript and JavaScript outside Effect-managed code.
requirement: >-
  Throw or reject only Error subclasses, never strings or plain objects. When
  wrapping a caught failure, preserve it through the cause option rather than
  string concatenation. Normalize non-Error throws once at a third-party
  boundary with a comment explaining the source.
applicability: |-
  Applies to non-Effect error handling.
  Exception: Runtimes or TypeScript lib settings without Error.cause, and
  framework boundaries that require a specific error shape. Effect code follows
  the existing tagged-error traits.
paths:
  - "*.ts"
  - "*.tsx"
  - "*.mts"
  - "*.cts"
  - "*.js"
  - "*.mjs"
  - "*.cjs"
  - "*.jsx"
purpose: Non-Errors lose stack traces, and dropped causes destroy the original failure context.
---
```diff
-try {
-  loadConfig()
-} catch (error) {
-  throw "bad input"
-}
+try {
+  loadConfig()
+} catch (error) {
+  throw new ParseError("invalid input", { cause: error })
+}
```

Sources: [Google TypeScript Style Guide: Only Throw Errors](https://google.github.io/styleguide/tsguide.html), [MDN: Error.cause](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Error/cause), [typescript-eslint: only-throw-error](https://typescript-eslint.io/rules/only-throw-error/).
