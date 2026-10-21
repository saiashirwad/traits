---
name: effect-platform-services
purpose: Make platform dependencies injectable, testable, and portable without breaking synchronous tooling contracts.
subject: Filesystem, path, working-directory, and child-process access.
requirement: "Use the Effect alternative for Node APIs in ordinary application
  code: node:fs and node:fs/promises use the FileSystem service; node:path uses
  the Path service; process.cwd() uses path.resolve() on the injected Path
  service; node:child_process uses effect/process with ChildProcess and
  ChildProcessSpawner. Access these services through Effect dependency injection
  so behavior remains testable and platform-independent. At synchronous tooling
  boundaries, such as Oxlint rule callbacks, native synchronous Node APIs may
  remain when needed to preserve the framework’s synchronous execution contract.
  Keep this exception confined to the tooling boundary rather than using it to
  bypass Effect in application code."
applicability: |-
  Applies when accessing filesystem, paths, working directories, or child processes in application code or synchronous tooling boundaries.
  Exception: When all native Node API usage under assessment is confined to synchronous tooling boundaries and is needed to preserve the framework's synchronous execution contract.
paths:
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.tsx"
  - "*.js"
  - "*.mjs"
  - "*.cjs"
  - "*.jsx"
---
```diff
-const text = await fs.readFile(file, "utf8")
+const fs = yield* FileSystem.FileSystem
+const text = yield* fs.readFileString(file)
```
