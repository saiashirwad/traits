---
name: py-concurrency-model
subject: Threads, processes, and asyncio in Python.
requirement: >-
  Use threads or ThreadPoolExecutor for blocking I/O, processes or
  ProcessPoolExecutor for CPU-bound work, and asyncio for high-concurrency I/O.
  Inside async def, never call blocking time.sleep, open, synchronous HTTP, or
  CPU-heavy code directly; await an async equivalent or offload via
  asyncio.to_thread or run_in_executor.
applicability: |-
  Exception: Free-threaded builds, C extensions that release the GIL,
  subprocess-based CPU work, and a single short blocking call during startup.
paths:
  - "*.py"
purpose: The GIL serializes Python bytecode, and a blocking call stalls every task on the event loop.
---
```diff
-async def load(path: Path) -> str:
-    time.sleep(1)
-    return open(path).read()
+async def load(path: Path) -> str:
+    await asyncio.sleep(1)
+    return await asyncio.to_thread(path.read_text, encoding="utf-8")
```

The blocking version is `time.sleep(1)` followed by `path.read_text(encoding="utf-8")` inside `async def`.

Sources: [asyncio: Running Blocking Code](https://docs.python.org/3/library/asyncio-dev.html#running-blocking-code), [Concurrency Overview](https://docs.python.org/3/library/concurrency.html), [GIL](https://docs.python.org/3/glossary.html#term-GIL), [Hynek: Waiting in asyncio](https://hynek.me/articles/waiting-in-asyncio/).
