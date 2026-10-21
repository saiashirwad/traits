---
name: jsx-same-named-prop-spreads
purpose: Reduce repetitive JSX prop forwarding while keeping special and computed props explicit.
subject: Ordinary React JSX props receiving same-named variables.
requirement: "In React JSX, prefer object-spread shorthand for ordinary props
  whose values are same-named variables: {...{ step, index }} instead of
  step={step} index={index}. Keep React’s special key prop explicit, and use
  explicit assignments for renamed or computed values."
applicability: |-
  Applies when passing same-named variables as ordinary React JSX props.
paths:
  - "*.tsx"
  - "*.jsx"
---
```diff
-<Step key={id} step={step} index={index} />
+<Step key={id} {...{ step, index }} />
```
