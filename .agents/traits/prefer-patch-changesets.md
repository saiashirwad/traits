---
name: prefer-patch-changesets
purpose: Keep routine release increments conservative and reserve larger increments for justified changes.
subject: Changesets and release increment selection.
requirement: Prefer patch changesets by default. Use a minor changeset only when
  there is a compelling reason for a minor release.
applicability: |-
  Applies when authoring changesets or choosing a release increment.
paths:
  - "*.md"
---
```diff
 ---
-"traits": minor
+"traits": patch
 ---
```
