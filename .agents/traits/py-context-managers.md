---
name: py-context-managers
subject: Ownership of files, sockets, locks, sessions, and temp directories in Python.
requirement: >-
  Acquire resource-owning objects with with or async with, or ExitStack, so
  cleanup is exception-safe. Do not hand-roll open and close or try/finally
  when a context manager exists. Specify encoding="utf-8" for text files unless
  the platform default is intentional.
applicability: |-
  Exception: Resources returned to a caller (document ownership),
  framework-managed sessions, objects without a context manager, and APIs
  requiring a raw str or bytes path.
guidance: Prefer pathlib for path manipulation when it does not obscure an OS-level API.
paths:
  - "*.py"
purpose: Deterministic release on every exit path prevents descriptor, lock, and connection leaks.
---
```diff
-file = open(path)
-data = file.read()
-file.close()
+with open(path, encoding="utf-8") as file:
+    data = file.read()
```

For small, bounded reads, `Path(path).read_text(encoding="utf-8")` is also a context-managed read; use streaming for unbounded input.

Sources: [Python Data Model: Context Managers](https://docs.python.org/3/reference/datamodel.html#context-managers), [contextlib](https://docs.python.org/3/library/contextlib.html), [Google Python Style Guide: Files and Sockets](https://google.github.io/styleguide/pyguide.html#311-files-sockets-and-similar-stateful-resources), [pathlib](https://docs.python.org/3/library/pathlib.html).
