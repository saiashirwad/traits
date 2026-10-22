---
name: go-idiomatic-naming
subject: Go identifiers, receivers, packages, and exported names.
requirement: >-
  Receivers are one or two letters derived from the type and consistent; never
  this or self. Exported names do not use a Get prefix unless it is the
  established name for a getter-like operation, such as a cache or store
  access, or required by an interface. Initialisms preserve case, such as ID,
  URL, HTTP, and appID. Package names are lowercase, underscore-free,
  meaningful, and avoid stutter with exported names.
applicability: |-
  Exception: Generated or protobuf code, existing public API compatibility, and
  third-party spellings such as gRPC.
paths:
  - "*.go"
purpose: Go names are read at call sites; short receivers, stable initialisms, and non-stuttering packages reduce noise.
---
```diff
-func (this *Client) GetURL() string { ... }
-package util
-type UtilConfig struct { ... }
+func (c *Client) URL() string { ... }
+package auth
+type Config struct { ... }
```

Sources: [Go Code Review Comments: Receiver Names](https://go.dev/wiki/CodeReviewComments#receiver-names), [Go Code Review Comments: Initialisms](https://go.dev/wiki/CodeReviewComments#initialisms), [Go Code Review Comments: Package Names](https://go.dev/wiki/CodeReviewComments#package-names), [Go Blog: Package Names](https://go.dev/blog/package-names).
