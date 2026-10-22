---
name: go-return-errors-not-fatal
subject: Failure signaling in Go libraries and applications.
requirement: >-
  Return an error for ordinary failures. Reserve panic for unrecoverable
  invariants, Must constructors, and initialization. Reserve log.Fatal and
  os.Exit for process entry points such as main and TestMain. Library code must
  not call log.Fatal or os.Exit for expected failures.
applicability: |-
  Exception: Must helpers during initialization, impossible-state checks, and
  test code. init may panic for an unrecoverable setup failure but should not
  call log.Fatal.
paths:
  - "*.go"
purpose: Callers can retry or degrade, deferred cleanup still runs, and tests are not killed by a library.
---
```diff
-func Load(path string) *Config {
-    data, err := os.ReadFile(path)
-    if err != nil {
-        log.Fatal(err)
-    }
-    ...
-}
+func Load(path string) (*Config, error) {
+    data, err := os.ReadFile(path)
+    if err != nil {
+        return nil, fmt.Errorf("read %s: %w", path, err)
+    }
+    ...
+}
```

Sources: [Google Go Style Decisions: Don't Panic](https://google.github.io/styleguide/go/decisions#dont-panic), [Go Code Review Comments: Don't Panic](https://go.dev/wiki/CodeReviewComments#dont-panic), [Uber Go Style Guide: Exit in Main](https://github.com/uber-go/guide/blob/master/style.md#exit-in-main).
