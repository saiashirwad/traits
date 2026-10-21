---
name: domain-specific-parsing-errors
purpose: Give callers meaningful recovery choices and actionable diagnostics without losing the original failure cause.
subject: Recoverable domain errors at parsing boundaries.
requirement: At parsing boundaries with recoverable domain-specific failures,
  use schema-backed tagged errors rather than Effect.try's generic UnknownError.
  Give distinct custom domain failure situations separate tags instead of a
  catch-all error distinguished only by reason strings. Include only actionable
  client context, such as an affected ID, path, source position, or limit;
  preserve the original cause where available without copying full inputs or
  adding unrelated or redundant fields. Retain SchemaError for input-schema
  validation failures, and do not wrap unrelated built-in operational errors
  merely for consistency.
applicability: |-
  Applies when defining recoverable custom domain errors or modeling parsing failures.
  Exception: When the parsing failure is already represented by SchemaError from input-schema validation.
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
---
```diff
-const parsed = Effect.try(() => parseDocument(text))
+const parsed = Effect.try({ try: () => parseDocument(text), catch: (cause) => InvalidDocument.make({ path, cause }) })
```
