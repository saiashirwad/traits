---
name: naming-intention-revealing
subject: Names of functions, types, variables, booleans, and modules.
requirement: >-
  Name the concept or effect, not the type or mechanism. Predicates read as
  assertions, such as isExpired or hasAccess. Functions name what they do, such
  as sendInvoice rather than doSend. Use one term for one concept. Avoid
  abbreviations except conventional short names in the language or domain, such
  as i, err, ctx, req, resp, and cfg, and established acronyms such as HTTP, ID,
  and URL.
applicability: |-
  Exception: Wire, serialization, and FFI names that must match an external
  schema; names fixed by a language, framework, protocol, or interface; public
  API compatibility; and conversion or factory names such as toString, parseInt,
  fromJSON, and newUser.
purpose: Names are the main compression mechanism in code and the first documentation a reader sees.
---
```diff
-proc(u, f)
-if u.st === 1 { ... }
+activateUser(user, sendWelcomeEmail)
```

Sources: [Google Engineering Practices: Naming](https://google.github.io/eng-practices/review/reviewer/looking-for.html#naming), [PEP 8: Naming Conventions](https://peps.python.org/pep-0008/#naming-conventions), [Effective Go: Names](https://go.dev/doc/effective_go#names).
