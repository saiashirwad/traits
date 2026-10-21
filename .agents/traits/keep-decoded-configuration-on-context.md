---
name: keep-decoded-configuration-on-context
purpose: Preserve a clear source of configuration truth and distinguish decoded settings from derived runtime state.
subject: Decoded configuration and derived runtime values exposed through an
  Effect context.
requirement: When an Effect context exposes decoded configuration, keep the
  configuration object on the context and read unchanged settings through it
  rather than copying individual settings into separate context fields. Keep
  derived runtime values, such as normalized paths and semaphores, as separate
  context fields.
applicability: |-
  Applies when exposing decoded configuration and derived runtime values through an Effect context.
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
---
```diff
-return { concurrency: config.concurrency, semaphore }
+return { config, semaphore }
```
