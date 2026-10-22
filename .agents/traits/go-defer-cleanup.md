---
name: go-defer-cleanup
subject: Cleanup of resources acquired in Go functions.
requirement: >-
  Defer cleanup immediately after acquiring a resource, before intervening work
  that can fail. In loops, use a per-iteration function or closure so defers do
  not accumulate. Check Close errors for writers and return them when they can
  affect correctness.
applicability: |-
  Applies to files, locks, connections, and other resources with cleanup.
  Exception: Benchmarks where defer cost is measured, and resources whose
  lifetime intentionally exceeds the function.
paths:
  - "*.go"
purpose: Guarantee release on every return path and keep close next to open.
---
```diff
-file, err := os.Create(path)
-if err != nil {
-    return err
-}
-workThatCanFail()
-file.Close()
+file, err := os.Create(path)
+if err != nil {
+    return err
+}
+defer file.Close()
+workThatCanFail()
```

Sources: [Effective Go: Defer](https://go.dev/doc/effective_go#defer), [Go Blog: Defer, Panic, and Recover](https://go.dev/blog/defer-panic-and-recover), [Uber Go Style Guide: Defer to Clean Up](https://github.com/uber-go/guide/blob/master/style.md#defer-to-clean-up).
