---
name: catch-tags-for-tagged-errors
purpose: Keep tagged-error recovery consistent and make related handlers easy to extend together.
subject: Tagged-error recovery in Effect programs.
requirement: 'Always prefer Effect.catchTags over Effect.catchTag for
  tagged-error recovery, including when handling only one error tag:
  Effect.catchTags({ UnknownError: handler }), not
  Effect.catchTag("UnknownError", handler). Preserve the existing handlers and
  error propagation behavior.'
applicability: |-
  Applies when recovering from tagged Effect errors.
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
---
```diff
-Effect.catchTag("UnknownError", handler)
+Effect.catchTags({ UnknownError: handler })
```
