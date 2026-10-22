---
name: clarity-over-cleverness
subject: Expression-level complexity and clever formulations.
requirement: >-
  Prefer the formulation a reader can verify at a glance. Flag nested ternaries
  of depth two or more, nested comprehensions of depth two or more, bit
  manipulation without a named constant or comment, and application-level
  metaprogramming that adds concepts rather than removing special cases.
  Elegance means fewer concepts and cases, not fewer lines.
applicability: |-
  Exception: Domain notation such as regular expressions, SQL, and mathematics;
  benchmarked hot kernels; standard idioms and generated bindings; idiomatic
  macros and decorators.
purpose: Cleverness shifts work from the author to every future reader.
---
```diff
-const label = a ? (b ? "both" : "a") : (b ? "b" : "none")
+let label = "none"
+if (a && b) label = "both"
+else if (a) label = "a"
+else if (b) label = "b"
```

Sources: [Go Proverbs](https://go-proverbs.github.io/), [PEP 20](https://peps.python.org/pep-0020/).
