---
name: py-stream-large-io
subject: Reading files and streams in Python.
requirement: >-
  Process potentially large files and streams incrementally by iterating the
  file object, reading chunks, or using shutil.copyfileobj; do not call an
  unbounded read on a file or response when no size bound is visible. Treat
  unknown size as inconclusive, and bound input size where a format requires a
  full read.
applicability: |-
  Exception: Small config files, formats requiring a full parse such as
  json.load or an XML tree when size is bounded, and memory-mapped access.
paths:
  - "*.py"
purpose: Whole-input reads scale memory with input and can exhaust the process on untrusted data.
---
```diff
-data = open(path).read()
+with Path(path).open(encoding="utf-8") as file:
+    for line in file:
+        process(line)
```

Sources: [io](https://docs.python.org/3/library/io.html), [pathlib](https://docs.python.org/3/library/pathlib.html), [Google Python Style Guide: Files and Sockets](https://google.github.io/styleguide/pyguide.html#311-files-sockets-and-similar-stateful-resources).
