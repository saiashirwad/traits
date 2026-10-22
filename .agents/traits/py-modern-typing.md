---
name: py-modern-typing
subject: Generic and union type annotations in modern Python.
requirement: >-
  On Python 3.9 or newer, or under from __future__ import annotations,
  annotate with built-in generics and collections.abc: list[str], dict[str,
  int], Callable[[int], str], rather than typing.List, typing.Dict, or
  typing.Callable. On Python 3.10 or newer, prefer X | None over Optional[X]
  and A | B over Union[A, B]. Keep Any, Literal, TypedDict, Protocol, Self,
  and TypeAlias.
applicability: |-
  Exception: Code supporting older Python without future annotations, and
  runtime annotation consumers that mishandle PEP 604 unions. When the
  project's minimum version is not visible in the supplied context, treat the
  version-specific clause as inconclusive rather than a violation.
paths:
  - "*.py"
  - "*.pyi"
purpose: Simpler annotations, no deprecated aliases, and consistent syntax that tools and readers both understand.
---
```diff
-from typing import Optional
-
-def find(ids: list[int]) -> Optional[dict[str, int]]:
+def find(ids: list[int]) -> dict[str, int] | None:
     ...
```

Sources: [PEP 585](https://peps.python.org/pep-0585/), [PEP 604](https://peps.python.org/pep-0604/), [Ruff: non-pep585-annotation (UP006)](https://docs.astral.sh/ruff/rules/non-pep585-annotation/), [Ruff: non-pep604-annotation-union (UP007)](https://docs.astral.sh/ruff/rules/non-pep604-annotation-union/).
