---
name: delete-dead-speculative-code
subject: Unused code and premature generalization.
requirement: >-
  Remove code that is visibly unreachable or unused within the supplied
  context: local or unexported functions, parameters, fields, if-false branches,
  and compatibility shims for unsupported versions. Public and exported items
  are out of scope. Do not add a type parameter, extension point, configuration
  flag, or interface until a present caller exercises it.
applicability: |-
  Applies only to local, private, or crate-private code visible in the supplied
  context. A public or exported item is not dead merely because no caller
  appears in the chunk.
  Exception: Published library API, serialization and FFI shapes, composition
  roots, test doubles, framework-discovered entry points such as route
  decorators, CLI subcommands, plugin hooks, dependency-injection registrations
  and migrations, signatures fixed by an interface or callback, __all__
  re-exports, and platform-gated code.
purpose: Dead code is still read and trusted; speculative abstraction adds concepts without evidence.
---
```diff
-// async function oldParse() { ... }
-function unusedHelper() { ... }
+// Removed by the current change.
```

Sources: [Refactoring: Remove Dead Code](https://refactoring.com/catalog/removeDeadCode.html), [Google Engineering Practices: Complexity](https://google.github.io/eng-practices/review/reviewer/looking-for.html#complexity), [matklad: Concrete Abstraction](https://matklad.github.io/2020/08/15/concrete-abstraction.html).
