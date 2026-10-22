---
name: rust-json-value-not-domain-model
subject: serde_json::Value or HashMap<String, Value> used as internal Rust domain data.
requirement: >-
  Deserialize external JSON into typed structs and enums at the I/O boundary,
  with serde(deny_unknown_fields) where the shape is closed, flatten is not
  used, and forward compatibility is not required. Keep serde_json::Value only
  for genuinely open payloads such as proxies, JSON-RPC params, or
  format-preserving tools. Do not index Value with string keys and unwrap in
  domain logic.
applicability: |-
  Exception: Pass-through tools that must preserve unknown fields,
  forward-compatible configuration, dynamic configuration merging, and editor
  or AST-style data.
paths:
  - "*.rs"
purpose: Keep type errors at the boundary instead of scattering runtime unwraps through domain code.
---
```diff
-let id = value["user"]["id"].as_str().unwrap();
+let event: Event = serde_json::from_str(&body)?;
+let id = &event.user.id;
```

Sources: [serde_json::Value](https://docs.rs/serde_json/latest/serde_json/enum.Value.html), [Rust API Guidelines: C-CUSTOM-TYPE](https://rust-lang.github.io/api-guidelines/type-safety.html#c-custom-type), [Effective Rust: Reflection](https://www.lurklurk.org/effective-rust/reflection.html).
