---
name: effect-retry-and-schedules
purpose: Keep retries bounded and composable while avoiding bespoke timing and attempt-management logic.
subject: Retry and backoff behavior in Effect programs.
requirement: 'Never manually implement retry or backoff logic. Use Effect’s
  built-in retry and scheduling APIs when retries are warranted:
  `request.pipe(Effect.retry({ times: 3, while: isTransient, schedule:
  Schedule.exponential("100 millis") }))`, not a retry loop with sleeps. Apply
  retries conservatively, with bounded attempts and appropriate failure
  filtering.'
applicability: |-
  Applies when adding retry or backoff behavior to Effect operations.
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
---
```diff
-for (let attempt = 0; attempt < 3; attempt++) { /* retry request and sleep manually */ }
+const retried = request.pipe(Effect.retry({ times: 3, while: isTransient, schedule: Schedule.exponential("100 millis") }))
```
