---
name: rust-enum-before-trait-object
subject: Closed alternatives modeled with traits and trait objects.
requirement: >-
  Model a closed set of alternatives as an enum and match exhaustively. Use dyn
  Trait for genuinely open sets such as plugins, user-supplied implementations,
  or type erasure, and generics for static dispatch over a caller-chosen type.
  Do not introduce one trait per struct to mimic a class hierarchy.
applicability: |-
  Applies to internal dispatch where the complete set of alternatives is
  visible in the supplied context.
  Exception: Object-safe extension points, heterogeneous collections built at
  runtime, recursive trait APIs, and public API stability that requires type
  erasure.
paths:
  - "*.rs"
purpose: Enums give exhaustive matching, avoid vtable overhead, and keep variants composable.
---
```diff
-trait Label { fn label(&self) -> &'static str; }
-fn labels(items: &[Box<dyn Label>]) -> Vec<&'static str> { ... }
+enum Level { Low, High }
+fn labels(items: &[Level]) -> Vec<&'static str> {
+    items.iter().map(|level| match level {
+        Level::Low => "low",
+        Level::High => "high",
+    }).collect()
+}
```

Sources: [Effective Rust: Generics](https://www.lurklurk.org/effective-rust/generics.html), [The Rust Book: Trait Objects](https://doc.rust-lang.org/book/ch17-02-trait-objects.html), [Rust API Guidelines: C-OBJECT](https://rust-lang.github.io/api-guidelines/flexibility.html#c-object).
