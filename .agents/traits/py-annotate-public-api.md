---
name: py-annotate-public-api
subject: Type annotations on public Python functions, methods, and classes.
requirement: >-
  In a module that uses type annotations, every public function, method, and
  class annotates all parameters and the return type. For classes, annotate
  __init__ and __new__ parameters. Use Any only when a type is genuinely
  unknowable, and prefer ParamSpec or Concatenate over *args and **kwargs typed
  as Any. Public names are those without a leading underscore.
applicability: |-
  Exception: Tests and scripts, self and cls, __init__ return values, decorated
  functions whose signatures are rewritten, and untyped legacy modules.
paths:
  - "*.py"
  - "*.pyi"
purpose: Type checkers, IDEs, and callers get a verifiable contract, and interface gaps surface before runtime.
---
```diff
-def load(path, strict=False):
+def load(path: Path, *, strict: bool = False) -> Config:
     ...
```

Sources: [PEP 484](https://peps.python.org/pep-0484/), [Google Python Style Guide: Type Annotations](https://google.github.io/styleguide/pyguide.html#319-type-annotations), [Python typing](https://docs.python.org/3/library/typing.html).
