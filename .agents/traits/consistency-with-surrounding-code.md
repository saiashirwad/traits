---
name: consistency-with-surrounding-code
subject: Local style and idiom not fixed by a formatter or linter.
requirement: >-
  Where formatter, linter, and written guidance are silent, follow the pattern
  visible in the supplied context for naming, error style, layout, test shape,
  and logging. Explicit project traits and language guides take precedence. Do
  not mix a drive-by reformat or rename with a behavior change.
applicability: |-
  Exception: The local pattern is a correctness or security defect, no dominant
  pattern is visible, or the language mandates a different idiom.
guidance: If the local pattern is wrong, change it everywhere in a separate change rather than adding a second convention.
purpose: Consistency lets readers predict unseen code and keeps review focused on behavior.
---
```diff
-func loadUser(id string) (*User, error) { ... }
-func fetchAccount(id string) (*Account, error) { ... }
+func loadUser(id string) (*User, error) { ... }
+func loadAccount(id string) (*Account, error) { ... }
```

Sources: [Google Engineering Practices: Consistency](https://google.github.io/eng-practices/review/reviewer/looking-for.html#consistency), [PEP 8: A Foolish Consistency](https://peps.python.org/pep-0008/#a-foolish-consistency-is-the-hobgoblin-of-little-minds), [Google Go Style Guide: Local Consistency](https://google.github.io/styleguide/go/guide#local-consistency).
