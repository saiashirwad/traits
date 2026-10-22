---
name: py-join-strings
subject: Building strings in Python loops.
requirement: >-
  Accumulate string fragments in a list and use "".join(...), or io.StringIO
  for incremental writes. Use f-strings or format for interpolation. Avoid
  repeated string addition over unbounded input.
applicability: |-
  Exception: Two to five fixed concatenations, bytes using b"".join as the
  recommended form, and SQL or HTML assembly, which belongs to a templating or
  parameterization layer.
paths:
  - "*.py"
purpose: join is guaranteed linear and states the intent; repeated addition relies on a CPython in-place optimization that breaks when the string escapes.
---
```diff
-out = ""
-for key, value in items:
-    out += f"{key}={value}\n"
+lines = []
+for key, value in items:
+    lines.append(f"{key}={value}")
+out = "\n".join(lines)
```

Sources: [str.join](https://docs.python.org/3/library/stdtypes.html#str.join), [io.StringIO](https://docs.python.org/3/library/io.html#io.StringIO), [Google Python Style Guide: Strings](https://google.github.io/styleguide/pyguide.html#310-strings).
