---
name: ts-release-resources
subject: Timers, listeners, subscriptions, and in-flight work with bounded lifetimes.
requirement: >-
  Within the lifetime scope visible in the supplied context, every timer,
  interval, event listener, subscription, observer, socket, and in-flight
  request created for a bounded lifetime has a matching release on teardown.
  Use AbortController or AbortSignal to cancel work, and remove listeners with
  the same function reference. Framework-managed cleanup such as React effects
  or Effect scopes should use the framework's mechanism.
applicability: |-
  Exception: Process-lifetime singletons, listeners intentionally alive until
  exit, and cleanup handled by a base class or framework.
paths:
  - "*.ts"
  - "*.tsx"
  - "*.mts"
  - "*.cts"
  - "*.js"
  - "*.mjs"
  - "*.cjs"
  - "*.jsx"
purpose: Retained callbacks keep closures alive and stale work keeps running after its owner is gone.
---
```diff
-const controller = new AbortController()
-fetch(url, { signal: controller.signal })
+const controller = new AbortController()
+window.addEventListener("resize", onResize, { signal: controller.signal })
+// teardown:
+controller.abort()
```

Sources: [MDN: AbortController](https://developer.mozilla.org/en-US/docs/Web/API/AbortController), [Google TypeScript Style Guide](https://google.github.io/styleguide/tsguide.html), [MDN: EventTarget.addEventListener](https://developer.mozilla.org/en-US/docs/Web/API/EventTarget/addEventListener).
