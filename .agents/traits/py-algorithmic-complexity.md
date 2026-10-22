---
name: py-algorithmic-complexity
subject: Collection membership and lookup in Python loops.
requirement: >-
  Prefer a better data structure over a faster inner statement: use set or dict
  for membership and keyed lookup, build indexes once outside loops, and avoid
  repeated scans or nested loops over the same collection. A micro-optimization
  that preserves quadratic behavior does not satisfy a performance goal.
applicability: |-
  Exception: n is small and a scan is clearer, order or duplicates are
  semantically required, items are unhashable, and inherently quadratic tasks
  such as pairwise comparison or all-pairs diffs.
paths:
  - "*.py"
purpose: Asymptotics dominate constant factors once inputs grow.
---
```diff
-for user in users:
-    if user.id in [admin.id for admin in admins]:
-        ...
+admin_ids = {admin.id for admin in admins}
+for user in users:
+    if user.id in admin_ids:
+        ...
```

Sources: [Python set types](https://docs.python.org/3/library/stdtypes.html#set-types-set-frozenset), [collections](https://docs.python.org/3/library/collections.html), [Python Performance Tips](https://wiki.python.org/python/performance/PythonSpeed/PerformanceTips.html).
