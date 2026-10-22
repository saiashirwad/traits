---
name: go-handle-errors
subject: Calls that return an error in Go.
requirement: >-
  Inspect every returned error and handle, return, or explicitly document why
  ignoring it is safe. Wrap with %w when callers need errors.Is or errors.As,
  and inspect wrapped errors with those helpers rather than == or string
  matching. Return error last. Error strings are lowercase, have no trailing
  punctuation, and include actionable context.
applicability: |-
  Exception: Deliberate best-effort cleanup of a read-only or idempotent
  resource, metrics and logging, direct io.EOF comparison, and %v when
  deliberately hiding an implementation detail. Writer Close errors follow
  go-defer-cleanup.
paths:
  - "*.go"
purpose: Ignored and flattened errors turn local failures into distant corruption and unactionable logs.
---
```diff
-if err == ErrNotFound { ... }
-return fmt.Errorf("Failed to load user: %v.", err)
+if errors.Is(err, ErrNotFound) { ... }
+return fmt.Errorf("load user %q: %w", id, err)
```

Sources: [Go 1.13 Errors](https://go.dev/blog/go1.13-errors), [Go Code Review Comments: Handle Errors](https://go.dev/wiki/CodeReviewComments#handle-errors), [Go Error Value FAQ](https://go.dev/wiki/ErrorValueFAQ).
