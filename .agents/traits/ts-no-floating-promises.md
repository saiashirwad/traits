---
name: ts-no-floating-promises
subject: Promise-returning expressions and async iteration callbacks in TypeScript and JavaScript.
requirement: >-
  Every promise whose rejection matters is awaited, returned, chained with
  catch, or explicitly voided with a comment explaining where rejection
  handling lives. Do not pass an async callback to forEach or reduce, or to
  filter, some, or every where a Promise object is used as a boolean. Use
  for...of with await, or map plus Promise.all for bounded concurrency.
applicability: |-
  Applies when the promise-returning type is visible in the supplied context;
  otherwise treat a bare call as inconclusive.
  Exception: Deliberate fire-and-forget work with explicit void and a documented
  rejection handler; Effect-managed programs follow the existing Effect traits.
paths:
  - "*.ts"
  - "*.tsx"
  - "*.mts"
  - "*.cts"
  - "*.js"
  - "*.mjs"
  - "*.cjs"
  - "*.jsx"
purpose: Unhandled rejections crash Node or vanish in browsers, and async callbacks in synchronous iterators hide ordering bugs.
---
```diff
-items.forEach(async (item) => {
-  await save(item)
-})
+for (const item of items) {
+  await save(item)
+}
```

Sources: [typescript-eslint: no-floating-promises](https://typescript-eslint.io/rules/no-floating-promises/), [typescript-eslint: no-misused-promises](https://typescript-eslint.io/rules/no-misused-promises/), [MDN: Promise](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Promise).
