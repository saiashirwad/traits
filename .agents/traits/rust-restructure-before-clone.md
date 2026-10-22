---
name: rust-restructure-before-clone
subject: Cloning added to satisfy the Rust borrow checker or to read one field.
requirement: >-
  Do not clone solely to satisfy the borrow checker. First narrow the borrow,
  split a method into field borrows, or restructure ownership. Use
  std::mem::take or replace only when the collection is meant to be consumed or
  replaced, and restore it when it must survive. Clone only when a second owned
  value is semantically intended. Do not call clone on Copy types.
applicability: |-
  Exception: Snapshot semantics, cheap Rc or Arc handle clones, clones required
  by a Clone bound, and cases where restructuring would make the code materially
  harder to follow.
paths:
  - "*.rs"
purpose: Avoid silent allocations and accidental state duplication while keeping ownership clear.
---
```diff
-for user in self.users.clone() {
-    self.report(&user);
-}
+for user in &self.users {
+    self.report(user);
+}
```

Sources: [Effective Rust: Borrows](https://www.lurklurk.org/effective-rust/borrows.html), [Clippy: redundant_clone](https://rust-lang.github.io/rust-clippy/master/index.html#redundant_clone), [std::mem::take](https://doc.rust-lang.org/std/mem/fn.take.html).
