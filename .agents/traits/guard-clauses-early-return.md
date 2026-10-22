---
name: guard-clauses-early-return
subject: Preconditions, invalid input, and terminal branches in ordinary functions.
requirement: >-
  Handle invalid, empty, or terminal cases first with an early return or raise,
  or continue in a loop, then keep the main path at base indentation. Do not
  place the happy path in an else branch after a branch that exits. Use a single
  compound guard when it reads more clearly than several exits.
applicability: |-
  Exception: Symmetric alternatives where both branches carry equal weight,
  exhaustive match or switch dispatch, and shared cleanup that would be
  duplicated across early exits (use defer or finally instead).
purpose: Reduce the state a reader must hold by making preconditions and the main path obvious.
---
```diff
-if (user) {
-  if (user.active) {
-    send(user)
-  }
-}
+if (!user?.active) return
+send(user)
```

Sources: [Go Code Review Comments: Indent Error Flow](https://go.dev/wiki/CodeReviewComments#indent-error-flow), [Refactoring: Replace Nested Conditional with Guard Clauses](https://refactoring.com/catalog/replaceNestedConditionalWithGuardClauses.html), [PEP 20](https://peps.python.org/pep-0020/).
