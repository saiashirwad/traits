---
name: terminology-in-site-copy
purpose: Give users consistent terminology that distinguishes a captured preference from the insight that prompted it.
subject: traits.sh user-facing copy and accessibility descriptions.
requirement: In traits.sh user-facing copy, including accessibility
  descriptions, call a saved or proposed reusable preference a “trait,” not a
  “rule.” Use “insight” for the lesson drawn from feedback before it is
  formulated as a trait.
applicability: |-
  Applies when writing traits.sh user-facing copy, including accessibility descriptions.
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
  - "*.html"
  - "*.md"
---
```diff
-<button>Save rule</button>
+<button>Save trait</button>
```
