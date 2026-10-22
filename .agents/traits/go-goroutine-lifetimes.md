---
name: go-goroutine-lifetimes
subject: Goroutines and unbounded fan-out in Go.
requirement: >-
  Every spawned goroutine must have a clear, reachable exit path, and its owner
  must be able to signal it and wait for completion. Bound fan-out over
  unbounded input with a worker pool, errgroup.SetLimit, or a semaphore.
  Prefer synchronous functions and let callers add concurrency.
applicability: |-
  Exception: Long-lived server workers and tickers that are documented and
  stopped at shutdown. A fixed small N is acceptable only when it is a
  compile-time constant rather than attacker-controlled input.
paths:
  - "*.go"
purpose: Blocked goroutines leak because the garbage collector cannot reclaim them, and in-flight work causes races and sends on closed channels.
---
```diff
-for _, url := range urls {
-    go fetch(url)
-}
+group, ctx := errgroup.WithContext(ctx)
+group.SetLimit(8)
+for _, url := range urls {
+    url := url // remove on Go 1.22+
+    group.Go(func() error { return fetch(ctx, url) })
+}
+return group.Wait()
```

Sources: [Go Code Review Comments: Goroutine Lifetimes](https://go.dev/wiki/CodeReviewComments#goroutine-lifetimes), [Go Blog: Pipelines](https://go.dev/blog/pipelines), [errgroup.Group.SetLimit](https://pkg.go.dev/golang.org/x/sync/errgroup#Group.SetLimit).
