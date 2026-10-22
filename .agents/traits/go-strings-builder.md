---
name: go-strings-builder
subject: String construction in Go loops.
requirement: >-
  Build strings in loops with strings.Builder and call Grow when the size is
  known, use bytes.Buffer for bytes, or use strings.Join for an existing slice.
  Avoid += and per-iteration fmt.Sprintf in loops.
applicability: |-
  Exception: Two or three fixed concatenations, constant expressions, and cases
  where strings.Join already expresses the operation.
paths:
  - "*.go"
purpose: Strings are immutable, so repeated concatenation copies the accumulated value and can become quadratic.
---
```diff
-var s string
-for _, part := range parts {
-    s += part + ","
-}
+var b strings.Builder
+b.Grow(totalLen)
+for _, part := range parts {
+    b.WriteString(part)
+    b.WriteByte(',')
+}
+s := b.String()
```

Sources: [strings.Builder](https://pkg.go.dev/strings#Builder), [bytes.Buffer](https://pkg.go.dev/bytes#Buffer), [strings.Join](https://pkg.go.dev/strings#Join).
