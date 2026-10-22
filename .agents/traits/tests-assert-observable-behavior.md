---
name: tests-assert-observable-behavior
subject: Test assertions and mocking boundaries.
requirement: >-
  Assert observable behavior through the public interface: return values,
  emitted events, persisted state, and rendered output. Mock only external
  boundaries such as network, clock, filesystem, and third-party services, plus
  important deterministic protocol or failure handling. Do not assert private
  state or internal collaborator calls unless the call itself is the contract,
  such as analytics, email, or an audit log. Model behavior under test needs
  real integrations, not mocks.
applicability: |-
  Exception: Contract or integration tests whose subject is the interaction
  itself, property-based tests, and benchmarks.
purpose: Implementation-coupled tests fail on safe refactors and pass when behavior is broken.
---
```diff
-it("works", () => {
-  save(row)
-  expect(repo.save).toHaveBeenCalledWith(row)
-})
+it("persists the invoice", async () => {
+  await save(row)
+  expect(await load(row.id)).toEqual(row)
+})
```

Sources: [Google Testing Blog: Test Behavior, Not Implementation](https://testing.googleblog.com/2013/08/testing-on-toilet-test-behavior-not.html), [Kent Beck: Test Desiderata](https://kentbeck.github.io/TestDesiderata/).
