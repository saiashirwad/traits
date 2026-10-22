---
name: tests-keep-cases-independent
subject: Test structure, case isolation, and determinism.
requirement: >-
  Name each test as scenario plus outcome using the framework's convention. Keep
  one reason to fail per test. Use arrange-act-assert order. Do not use branching
  or loops that compute expected values or select assertions; table-driven and
  parameterized cases are the standard way to repeat a case. Avoid sleeps,
  wall-clock dependence, network dependence, and order dependence.
applicability: |-
  Exception: Table-driven and parameterized cases, fail-fast assertion guards
  such as if err != nil { t.Fatal(err) }, property-based tests, and integration
  tests that must exercise real time or network.
purpose: A failing test should identify one behavior without carrying its own logic bugs or hidden order dependence.
---
```diff
-it("works", () => {
-  for (const row of rows) {
-    if (row.kind === "invoice") expect(parse(row)).toEqual(row)
-  }
-})
+it("rejects an expired token", () => {
+  expect(() => verify(expiredToken)).toThrow(TokenExpiredError)
+})
```

Sources: [Google Testing Blog: Don't Put Logic in Tests](https://testing.googleblog.com/2014/07/testing-on-toilet-dont-put-logic-in.html), [Kent Beck: Test Desiderata](https://kentbeck.github.io/TestDesiderata/), [pytest: Parametrize](https://docs.pytest.org/en/stable/how-to/parametrize.html).
