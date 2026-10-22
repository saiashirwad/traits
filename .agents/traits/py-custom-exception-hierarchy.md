---
name: py-custom-exception-hierarchy
subject: Domain error types raised by Python packages and libraries.
requirement: >-
  When defining custom domain errors, use specific subclasses named with an
  Error suffix. A shared base exception is recommended but not required. Do not
  raise bare Exception. Builtins remain correct for programming errors and
  protocol violations.
applicability: |-
  Exception: Scripts and tests may use builtins; NotImplementedError remains
  appropriate; thin wrappers around third-party exceptions may re-raise them.
paths:
  - "*.py"
purpose: Callers can catch domain failures precisely without catching unrelated bugs.
---
```diff
-raise Exception("invalid config")
+class MissingKeyError(ValueError):
+    """Raised when a required configuration key is absent."""
+
+raise MissingKeyError(key)
```

Sources: [Google Python Style Guide: Exceptions](https://google.github.io/styleguide/pyguide.html#24-exceptions), [PEP 8: Exception Names](https://peps.python.org/pep-0008/#exception-names), [Ruff: raise-vanilla-class (TRY002)](https://docs.astral.sh/ruff/rules/raise-vanilla-class/).
