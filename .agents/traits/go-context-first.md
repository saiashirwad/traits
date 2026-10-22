---
name: go-context-first
subject: context.Context use in Go APIs.
requirement: >-
  Accept context.Context as the first parameter of functions that perform
  cancellable or request-scoped work, pass it through to downstream calls, and
  never store it in a struct or define a custom context type. Derive timeouts
  and deadlines at the boundary, and use context.Background only at entry
  points.
applicability: |-
  Exception: Tests and TestMain, independent background lifecycles with no
  caller context, and interface signatures that prevent passing a context.
  Storing a context is acceptable only where such an interface prevents
  passing it through.
paths:
  - "*.go"
purpose: Cancellation, deadlines, and request values must flow through the call chain instead of being pinned to an object lifetime.
---
```diff
-type Server struct { ctx context.Context }
-func (s *Server) Do() error { ... }
+func (s *Server) Do(ctx context.Context) error {
+    return s.client.Fetch(ctx)
+}
```

Sources: [Go Code Review Comments: Contexts](https://go.dev/wiki/CodeReviewComments#contexts), [Google Go Style Decisions: Contexts](https://google.github.io/styleguide/go/decisions#contexts), [Go Blog: Context](https://go.dev/blog/context).
