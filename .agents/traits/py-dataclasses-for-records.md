---
name: py-dataclasses-for-records
subject: Python classes that primarily hold typed data.
requirement: >-
  Use @dataclass, or attrs or Pydantic when you need converters, validators, or
  features beyond dataclass support, for classes that primarily hold typed
  data, instead of hand-written __init__, __repr__, and __eq__. Use frozen=True
  for immutable value objects and field(default_factory=...) for mutable
  fields. Use __post_init__ or validators for invariants.
applicability: |-
  Exception: ORM models, classes with complex initialization, classes that need
  custom equality, and APIs that require dynamic attributes.
paths:
  - "*.py"
purpose: Remove boilerplate and get correct, type-driven equality and representation.
---
```diff
-class Point:
-    def __init__(self, x, y):
-        self.x = x
-        self.y = y
+@dataclass(frozen=True)
+class Point:
+    x: int
+    y: int
```

Sources: [dataclasses](https://docs.python.org/3/library/dataclasses.html), [PEP 557](https://peps.python.org/pep-0557/), [Hynek: import attrs](https://hynek.me/articles/import-attrs/).
