---
name: go-value-vs-pointer
subject: Value and pointer receivers, parameters, and range variables in Go.
requirement: >-
  Use value receivers and parameters for small immutable structs; use pointers
  for mutation, large structs, or types containing a mutex. Keep receiver types
  consistent. Do not pass *string or *io.Reader merely to avoid a copy. In a
  range over large structs, or when mutating elements, iterate by index or
  pointer instead of copying each value.
applicability: |-
  Exception: Small structs where value semantics improve clarity, methods fixed
  by an interface signature, and types that are already references such as maps,
  functions, and channels.
paths:
  - "*.go"
purpose: Avoid hidden copies and pointer chasing while keeping mutation semantics obvious.
---
```diff
-for _, row := range rows {
-    row.Status = "seen"
-}
+for i := range rows {
+    rows[i].Status = "seen"
+}
```

Sources: [Go Code Review Comments: Pass Values](https://go.dev/wiki/CodeReviewComments#pass-values), [Go Code Review Comments: Receiver Type](https://go.dev/wiki/CodeReviewComments#receiver-type), [The Go Spec: For Range](https://go.dev/ref/spec#For_range).
