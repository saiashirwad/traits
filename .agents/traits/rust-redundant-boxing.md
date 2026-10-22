---
name: rust-redundant-boxing
subject: Box used around values or collections that do not need indirection.
requirement: >-
  Do not box a value or collection unless boxing is required for recursion, a
  large enum variant, a trait object, a stable address, or an API that needs
  type erasure. Box<Vec<T>>, Box<String>, Box<&T>, and Box<Box<T>> add
  indirection and allocation without changing semantics. Use Box<[T]> only when
  a fixed-size owned slice is intended.
applicability: |-
  Exception: Recursive types, Box<dyn Trait>, boxed errors, large enum variants
  intentionally out-lined, and FFI layouts.
paths:
  - "*.rs"
purpose: Keep ownership shapes honest and avoid indirection that adds no capability.
---
```diff
-fn names(&self) -> Box<Vec<String>> {
-    Box::new(self.names.clone())
-}
+fn names(&self) -> &[String] {
+    &self.names
+}
```

Sources: [The Rust Book: Using Box to Point to Data on the Heap](https://doc.rust-lang.org/book/ch15-01-box.html), [Clippy: box_collection](https://rust-lang.github.io/rust-clippy/master/index.html#box_collection), [Clippy: redundant_allocation](https://rust-lang.github.io/rust-clippy/master/index.html#redundant_allocation).
