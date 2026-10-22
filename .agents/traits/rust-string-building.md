---
name: rust-string-building
subject: String assembly in Rust loops and formatting-heavy paths.
requirement: >-
  Build strings in loops by appending into a reused String with push_str or
  push, or with write! after importing std::fmt::Write, and reserve an
  estimated capacity. Avoid format! per iteration, repeated string addition,
  and nested format! inside formatting macros. Use format! for cold, one-shot
  messages where clarity wins.
applicability: |-
  Exception: Cold error construction, a few fixed concatenations, and constant
  expressions the compiler can fold.
paths:
  - "*.rs"
purpose: Avoid per-iteration allocation and quadratic copying in formatting paths.
---
```diff
-for item in items {
-    out.push_str(&format!("{item},"));
-}
+out.reserve(estimated_len);
+for item in items {
+    write!(out, "{item},").expect("writing to a String cannot fail");
+}
```

Sources: [Clippy: format_in_format_args](https://rust-lang.github.io/rust-clippy/master/index.html#format_in_format_args), [std::string::String](https://doc.rust-lang.org/std/string/struct.String.html), [Rust Performance Book: Heap Allocations](https://nnethercote.github.io/perf-book/heap-allocations.html).
