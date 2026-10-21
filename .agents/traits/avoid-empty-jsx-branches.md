---
name: avoid-empty-jsx-branches
purpose: Reduce rendering noise while preserving intentional layout, accessibility, and visible behavior.
subject: Conditional content in React JSX.
requirement: 'In React JSX, avoid unnecessary empty-content branches: render
  text directly when CSS already preserves its layout, and use a boolean
  condition with && when the alternative renders nothing, such as {status !==
  "Copy" && status} instead of {status === "Copy" ? "" : status}. Keep
  placeholders or explicit alternatives when they are needed for layout,
  accessibility, or behavior.'
applicability: |-
  Applies when writing conditional React JSX content or empty-content branches.
  Exception: When every placeholder or explicit alternative under assessment is required for layout, accessibility, or behavior rather than being an unnecessary empty-content branch.
paths:
  - "*.tsx"
  - "*.jsx"
---
```diff
-{status === "Copy" ? "" : status}
+{status !== "Copy" && status}
```
