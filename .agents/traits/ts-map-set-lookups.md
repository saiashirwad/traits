---
name: ts-map-set-lookups
subject: Repeated array scans and keyed lookups in TypeScript and JavaScript.
requirement: >-
  Avoid repeated or multiplied scans: when a loop scans an array with includes,
  find, some, or filter, or a scan's cost multiplies with another collection,
  build a Set or Map once and use sublinear lookup. Keep arrays when order
  matters or the collection is small and a scan is clearer.
applicability: |-
  Exception: Small fixed arrays, index-order requirements, plain objects that
  must serialize to JSON, and one-off lookups outside loops.
paths:
  - "*.ts"
  - "*.tsx"
  - "*.mts"
  - "*.cts"
  - "*.js"
  - "*.mjs"
  - "*.cjs"
  - "*.jsx"
purpose: Repeated linear scans dominate once collections grow.
---
```diff
-const selected = items.filter((item) => selectedIds.includes(item.id))
+const selectedSet = new Set(selectedIds)
+const selected = items.filter((item) => selectedSet.has(item.id))
```

Sources: [MDN: Map](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Map), [MDN: Set](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Set).
