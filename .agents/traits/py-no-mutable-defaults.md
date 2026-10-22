---
name: py-no-mutable-defaults
subject: Default argument values in Python function and method signatures.
requirement: >-
  Never use a mutable object such as a list, dict, set, or a call producing one
  as a default argument. Use None or a unique sentinel and initialize inside
  the function, or field(default_factory=...) in dataclasses. Immutable
  defaults such as tuple, str, int, and frozenset are fine. If None is a valid
  argument value, use a module-level sentinel.
applicability: |-
  Exception: Immutable defaults such as tuple, str, int, and frozenset.
paths:
  - "*.py"
purpose: Defaults are evaluated once at definition time, so mutable defaults leak state across calls and instances.
---
```diff
-def add(item, items=[]):
+def add(item, items=None):
+    if items is None:
+        items = []
     items.append(item)
     return items
```

Sources: [Google Python Style Guide: Default Argument Values](https://google.github.io/styleguide/pyguide.html#212-default-argument-values), [Ruff: mutable-argument-default (B006)](https://docs.astral.sh/ruff/rules/mutable-argument-default/), [dataclasses](https://docs.python.org/3/library/dataclasses.html).
