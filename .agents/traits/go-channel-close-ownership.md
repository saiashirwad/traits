---
name: go-channel-close-ownership
subject: Closing Go channels.
requirement: >-
  Only the sending side closes a data channel, and it closes it exactly once.
  Receivers never close. With multiple senders, wait for all senders through a
  WaitGroup or errgroup, then close from the owner. Never close a channel you
  did not create or are not the sole sender for; sending on a caller-provided
  channel is normal.
applicability: |-
  Exception: Cancellation channels closed by their owner as a broadcast signal.
paths:
  - "*.go"
purpose: Sending on a closed channel panics, and receiver-side close races with every sender.
---
```diff
-func consume(ch <-chan int) {
-    close(ch)
-}
+func produce(ch chan<- int) {
+    defer close(ch)
+    // send values
+}
```

Sources: [The Go Spec: Close](https://go.dev/ref/spec#Close), [Go Blog: Pipelines](https://go.dev/blog/pipelines), [Go Code Review Comments: Goroutine Lifetimes](https://go.dev/wiki/CodeReviewComments#goroutine-lifetimes).
