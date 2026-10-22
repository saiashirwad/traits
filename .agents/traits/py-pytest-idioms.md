---
name: py-pytest-idioms
subject: Test structure in pytest suites.
requirement: >-
  Use plain assert, pytest.raises(SpecificError) around the minimal call, and
  pytest.mark.parametrize for table-driven cases. Add match= when the message
  is part of the contract; otherwise assert the specific exception type. Use
  yield fixtures when teardown is needed. Avoid unittest-style assertEqual and
  assertTrue, and avoid loops over cases that hide which case failed.
applicability: |-
  Applies to pytest test suites.
  Exception: Projects standardized on unittest.TestCase, property-based tests,
  and one-off tests with no repeated cases.
paths:
  - "*.py"
purpose: Pytest's assertion rewriting gives useful diffs, and parametrization names each case and isolates failures.
---
```diff
-self.assertEqual(add(1, 2), 3)
+assert add(1, 2) == 3
 
-with pytest.raises(Exception):
+with pytest.raises(ValueError, match="empty"):
     parse("")
```

Sources: [pytest: Assertions](https://docs.pytest.org/en/stable/how-to/assert.html), [pytest: Parametrize](https://docs.pytest.org/en/stable/how-to/parametrize.html), [pytest: Fixtures](https://docs.pytest.org/en/stable/how-to/fixtures.html), [Ruff: pytest-raises-too-broad (PT011)](https://docs.astral.sh/ruff/rules/pytest-raises-too-broad/).
