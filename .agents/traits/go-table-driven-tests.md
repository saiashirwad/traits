---
name: go-table-driven-tests
subject: Go tests with multiple similar cases.
requirement: >-
  Tests with multiple similar cases use a table with named fields and t.Run
  subtests. Helpers that report failures call t.Helper(). Failures report the
  input, got, and want rather than only "failed". Do not put t.Fatal in the
  parent loop; call it inside the subtest.
applicability: |-
  Applies to unit tests with two or more similar cases.
  Exception: One-off or highly heterogeneous tests; assertion libraries are not
  required.
paths:
  - "*.go"
purpose: Adding cases stays cheap, each failure names its case, and helper failures point at the caller.
---
```diff
-func TestParseA(t *testing.T) { assertParse(t, "a", 1) }
-func TestParseB(t *testing.T) { assertParse(t, "b", 2) }
+tests := []struct {
+    name string
+    in   string
+    want int
+}{
+    {"a", "a", 1},
+    {"b", "b", 2},
+}
+for _, tt := range tests {
+    tt := tt // remove on Go 1.22+
+    t.Run(tt.name, func(t *testing.T) {
+        assertParse(t, tt.in, tt.want)
+    })
+}
```

Sources: [Go Blog: Subtests](https://go.dev/blog/subtests), [Go Code Review Comments: Useful Test Failures](https://go.dev/wiki/CodeReviewComments#useful-test-failures), [testing.T.Helper](https://pkg.go.dev/testing#T.Helper).
