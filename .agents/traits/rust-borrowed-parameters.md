---
name: rust-borrowed-parameters
subject: Rust function parameter and return types.
requirement: >-
  Accept &str, &[T], or &Path when a function only reads, rather than &String,
  &Vec<T>, or &PathBuf. Take ownership only when storing or consuming the
  value. Return borrowed data or impl Iterator when the function does not
  allocate.
applicability: |-
  Exception: API compatibility, trait methods that require concrete types, and
  hot internal call sites where a convenience generic hurts inference or compile
  time; prefer a concrete borrow there.
guidance: >-
  At public boundaries, consider impl AsRef<Path> or impl IntoIterator for
  caller convenience, delegating to a monomorphic inner function so hot
  internal code stays concrete.
paths:
  - "*.rs"
purpose: Callers avoid conversions and allocations, and the signature documents ownership.
---
```diff
-fn greet(name: &String) -> String { format!("hi {name}") }
-fn read(path: &PathBuf) -> io::Result<String> { ... }
+fn greet(name: &str) -> String { format!("hi {name}") }
+fn read(path: impl AsRef<Path>) -> io::Result<String> { ... }
```

Sources: [Rust API Guidelines: C-CALLER-CONTROL and C-GENERIC](https://rust-lang.github.io/api-guidelines/flexibility.html), [Clippy: ptr_arg](https://rust-lang.github.io/rust-clippy/master/index.html#ptr_arg), [Effective Rust: Borrows](https://www.lurklurk.org/effective-rust/borrows.html).
