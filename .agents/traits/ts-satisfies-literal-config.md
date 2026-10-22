---
name: ts-satisfies-literal-config
subject: Literal lookup tables keyed by a finite union in TypeScript.
requirement: >-
  Use satisfies T to validate a literal lookup table against a finite keyed
  type while preserving its precise keys and discriminants. Use a colon
  annotation when the table is intentionally widened or consumers must see
  exactly T. Do not use satisfies as a substitute for runtime validation of
  external data.
applicability: |-
  Exception: Exported constants intentionally widened, configuration objects
  that are not indexed by a literal key, and values assembled at runtime.
paths:
  - "*.ts"
  - "*.tsx"
  - "*.mts"
  - "*.cts"
purpose: Keep literal key and discriminant types narrow while still checking conformance to a finite keyed type.
---
```diff
-const routes: Record<RouteName, Handler> = { home: homeHandler }
+type RouteName = "home" | "about"
+const routes = { home: homeHandler, about: aboutHandler } satisfies Record<RouteName, Handler>
```

Sources: [TypeScript 4.9 Release Notes: satisfies](https://www.typescriptlang.org/docs/handbook/release-notes/typescript-4-9.html), [TypeScript Handbook: Object Types](https://www.typescriptlang.org/docs/handbook/2/objects.html).
