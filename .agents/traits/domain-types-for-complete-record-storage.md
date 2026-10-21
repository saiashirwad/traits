---
name: domain-types-for-complete-record-storage
purpose: Prevent storage contracts from drifting from domain models or silently dropping record content.
subject: Storage operations creating or replacing complete domain records.
requirement: For storage operations that create or replace complete domain
  records, accept the existing domain type instead of separate per-field
  parameters or a manually duplicated object type. Persist the complete record
  without dropping optional metadata or extension fields.
applicability: |-
  Applies when designing or implementing storage operations that create or replace complete domain records.
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
---
```diff
-const add = (name: TraitName, body: string) => persist({ name, body })
+const add = (trait: Trait) => persist(trait)
```
