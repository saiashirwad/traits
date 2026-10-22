---
name: go-consumer-side-interfaces
subject: Interface declaration and constructor return types in Go.
requirement: >-
  Define small interfaces in the package that consumes them, and return
  concrete types from constructors. Do not declare a producer-side interface
  beside its only implementation when no consumer in the supplied context
  imports it. Do not create an interface solely for mocking before a real
  consumer exists.
applicability: |-
  Exception: An exported interface that is the product or protocol,
  standard-library-shaped APIs, genuine polymorphism, and plugin boundaries.
paths:
  - "*.go"
purpose: Consumer-defined interfaces stay minimal, concrete returns preserve access to new methods, and premature abstraction freezes APIs.
---
```diff
-package store
-type Store interface { Get(...); Put(...); List(...) }
-func New() Store { ... }
+package store
+func New() *Store { ... }
+
+package billing
+type userGetter interface {
+    GetUser(ctx context.Context, id string) (*User, error)
+}
```

Sources: [Go Code Review Comments: Interfaces](https://go.dev/wiki/CodeReviewComments#interfaces), [Google Go Style Decisions: Interfaces](https://google.github.io/styleguide/go/decisions#interfaces), [Go Blog: When to Use Generics](https://go.dev/blog/when-generics).
