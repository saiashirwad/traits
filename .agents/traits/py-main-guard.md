---
name: py-main-guard
subject: Executable behavior in Python modules.
requirement: >-
  Put executable behavior in main() and invoke it only under if __name__ ==
  "__main__":. Importing a module must not run the program. Keep argument
  parsing and side effects inside main().
applicability: |-
  Exception: __main__.py, console-script entry points, notebooks, and modules
  with no executable behavior.
paths:
  - "*.py"
purpose: Importing a module must be safe for tests, documentation, and reuse.
---
```diff
-args = parse_args()
-run(args)
+def main() -> None:
+    run(parse_args())
+
+if __name__ == "__main__":
+    main()
```

Sources: [Google Python Style Guide: Main](https://google.github.io/styleguide/pyguide.html#317-main), [Python __main__](https://docs.python.org/3/library/__main__.html), [PEP 8](https://peps.python.org/pep-0008/).
