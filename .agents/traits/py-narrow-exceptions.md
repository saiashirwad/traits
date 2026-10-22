---
name: py-narrow-exceptions
subject: try and except blocks in Python.
requirement: >-
  Catch the narrowest expected exception type. Do not use bare except, or
  except Exception followed by pass. Reserve broad catches for isolation
  boundaries that log and re-raise or deliberately suppress, and use
  contextlib.suppress(SpecificError) for intentional ignores. When raising a
  different exception inside an except, use raise NewError(...) from err; use
  from None only when deliberately hiding implementation details.
applicability: |-
  Applies to exception handling in Python code.
  Exception: Process, worker, and plugin isolation boundaries; cleanup that must
  not mask the original failure.
paths:
  - "*.py"
purpose: Bare except also catches KeyboardInterrupt and SystemExit; broad catches hide bugs and mask typos.
---
```diff
-try:
-    cfg = load(path)
-except:
-    pass
+try:
+    cfg = load(path)
+except (OSError, ValueError) as err:
+    raise MissingKeyError(path) from err
```

Sources: [Google Python Style Guide: Exceptions](https://google.github.io/styleguide/pyguide.html#24-exceptions), [PEP 8: Exception Names](https://peps.python.org/pep-0008/#exception-names), [Ruff: bare-except (E722)](https://docs.astral.sh/ruff/rules/bare-except/), [Ruff: raise-without-from-inside-except (B904)](https://docs.astral.sh/ruff/rules/raise-without-from-inside-except/).
