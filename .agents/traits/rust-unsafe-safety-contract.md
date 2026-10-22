---
name: rust-unsafe-safety-contract
subject: Unsafe blocks, unsafe impls, and unsafe functions.
requirement: >-
  Every unsafe block and unsafe impl carries a SAFETY comment stating why its
  preconditions hold. Every unsafe fn documents caller obligations in a
  # Safety section. Use get_unchecked, raw pointer arithmetic, and unsafe impl
  Send or Sync only with a local proof; never as a speculative optimization.
applicability: |-
  Exception: Generated bindings may carry a file-level justification instead of
  repeating comments on every item.
paths:
  - "*.rs"
purpose: Make every discharged safety obligation reviewable and prevent undefined behavior from undocumented assumptions.
---
```diff
-let value = unsafe { *ptr.add(index) };
+// SAFETY: `ptr` comes from `values.as_ptr()` and `index < values.len()`.
+let value = unsafe { *ptr.add(index) };
```

Sources: [The Rust Reference: Unsafe Keyword](https://doc.rust-lang.org/reference/unsafe-keyword.html), [Rust API Guidelines: C-FAILURE](https://rust-lang.github.io/api-guidelines/documentation.html#function-docs-include-error-panic-and-safety-considerations-c-failure), [Clippy: undocumented_unsafe_blocks](https://rust-lang.github.io/rust-clippy/master/index.html#undocumented_unsafe_blocks).
