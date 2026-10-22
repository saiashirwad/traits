---
name: go-no-global-mutable-state
subject: Package-level initialization and mutable state in Go libraries.
requirement: >-
  Libraries must not use init or mutable package variables to perform I/O,
  read environment or configuration, or hold dependencies. Use explicit
  constructors and injected instances. Keep init deterministic and
  side-effect-light when it is required for registration.
applicability: |-
  Exception: Immutable sentinels and constants, codec or driver registration
  through side-effect imports, regexp.MustCompile of constant patterns, main
  wiring, concurrency-safe caches and pools, and stateless shared clients with
  documented invariants.
paths:
  - "*.go"
purpose: Globals hide dependencies, break test isolation, prevent multiple instances, and make initialization failures fatal.
---
```diff
-var db *sql.DB
-func init() {
-    db, _ = sql.Open("pg", os.Getenv("DSN"))
-}
+type Server struct { db *sql.DB }
+func New(db *sql.DB) *Server { return &Server{db: db} }
```

Sources: [Google Go Best Practices: Global State](https://google.github.io/styleguide/go/best-practices#global-state), [Uber Go Style Guide: Avoid init](https://github.com/uber-go/guide/blob/master/style.md#avoid-init), [Effective Go: Initialization](https://go.dev/doc/effective_go#initialization).
