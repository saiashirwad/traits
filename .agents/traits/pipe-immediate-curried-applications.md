---
name: pipe-immediate-curried-applications
purpose: Make the transformed value and curried operation readable in forward execution order.
subject: Immediate application of functions returned by curried calls.
requirement: "When a function call returns a function that is immediately
  applied to a value, use `pipe(value, fn(args))`, never `fn(args)(value)`.
  Inspect nested call expressions even across line breaks and inside `yield*`:
  use `yield* pipe(decomposed, S.encodeEffect(schema))`, not `yield*
  S.encodeEffect(schema)(decomposed)`. This applies even to one transformation
  and when no data-first overload exists. It does not prohibit calling a
  previously stored function or using a genuine data-first overload such as
  `Array.map(items, f)`."
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
-const decoded = yield* S.decodeUnknownEffect(schema)(input)
+const decoded = yield* pipe(input, S.decodeUnknownEffect(schema))
```
