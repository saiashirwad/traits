---
name: comments-explain-why
subject: Source comments, TODOs, and commented-out code.
requirement: >-
  Comment rationale, invariants, external constraints, units, and non-obvious
  tradeoffs that code cannot express. Do not narrate the next statement. Delete
  commented-out code. A TODO names a tracked issue or a concrete removal
  condition such as a date or event, preferring an issue over an owner.
applicability: |-
  Exception: Doc comments that state an API contract, machine-read directives
  such as build tags and SAFETY notes, and complex algorithms where a brief
  what-comment materially aids comprehension.
purpose: Stale or redundant comments mislead, while why-comments preserve decisions code cannot carry.
---
```diff
-i += 1  # increment i
-// old: return cached
+// Retry three times: the vendor returns 502 during rolling deploys (INC-2291).
+// TODO(alice): remove after the v2 migration (#123).
```

Sources: [Google Engineering Practices: Comments](https://google.github.io/eng-practices/review/reviewer/looking-for.html#comments), [PEP 8: Comments](https://peps.python.org/pep-0008/#comments), [Google Python Style Guide: TODO Comments](https://google.github.io/styleguide/pyguide.html#312-todo-comments).
