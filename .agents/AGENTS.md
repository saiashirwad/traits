# Project Guidelines

All paths below are relative to the repository root.

## Workspace

- `traits/`: Effect-based MCP server and trait SDK.
- `traits.sh/`: static Vite + React site; deployment is defined by root `alchemy.run.ts`.
- Use pnpm and Node-compatible tooling; do not introduce Bun APIs.
- Use `pnpm --filter traits.sh dev` for the local Vite server and `pnpm --filter traits.sh build` to build static assets.
- Deployment runs from the root with `pnpm exec alchemy deploy alchemy.run.ts --stage <stage>`. Do not deploy, destroy, or provision resources without authorization; credentials/profile and stage must be explicit.

## Verification

- Run checks from the root: `pnpm exec vp check` for formatting/lint/types; `pnpm build` after TypeScript changes (`--force` after config changes).
- Run affected suites: `pnpm test:oxlints` and `pnpm test:traits`. The traits suite may be empty; do not call an empty suite a behavior-test pass. The static site has no test project. `pnpm test` runs checks, builds, and suites; docs-only edits need no build/tests.
- After dependency changes, run `pnpm install`; after traits exports/publishing changes, run `pnpm --filter traits pack` and verify the tarball's imports/types in a consumer.
- After frontend changes, run `pnpm --filter traits.sh build`. No deployment is required for this verification.
- Report failures, blocked checks, and empty/skipped suites accurately.

## CI Credentials

- Whenever deployment configuration changes or new environment variables are introduced, update `bootstrap.ts` to provision the required GitHub Actions secrets/variables, rerun bootstrap, and expose them in the workflow's deployment environment before deploying from CI.
- Bootstrap with `CLOUDFLARE_ACCOUNT_ID=<account-id> pnpm exec alchemy deploy bootstrap.ts --stage bootstrap`, using an Alchemy profile authorized to create Cloudflare account tokens and manage GitHub Actions secrets/variables.
- Deployment steps must expose `CLOUDFLARE_API_TOKEN` from `${{ secrets.CLOUDFLARE_API_TOKEN }}` and `CLOUDFLARE_ACCOUNT_ID` from `${{ vars.CLOUDFLARE_ACCOUNT_ID }}`.

## Effect

- Use `Effect.fn` with a concise span name for reusable Effect operations: `Effect.fn("<name>")(function* () { ... })` rather than `Effect.fn(function* () { ... })`. Retain `Effect.gen` for inline effect values and initialization.
- Prefer `Effect.fn` / `Effect.fnUntraced` over functions that only return `Effect.gen`, including callbacks such as `items.map(Effect.fn("processItem")(function* (item) { ... }))`.
- Apply transformations through additional function arguments to the wrapper. These receive the effect and the original arguments, so argument-dependent services do not require an outer arrow function:

  ```ts
  const process = Effect.fn("process")(
    function* (item: Item) {
      /* ... */
    },
    (effect, item) => Effect.provideService(effect, TheService, item)
  )
  ```

  Use the same form with `Effect.fnUntraced` in library code, without a span name.

- An outer callback returning `Effect.gen(...).pipe(...)` is appropriate when it needs to capture callback-local values for the pipeline, e.g. `(item) => Effect.gen(function* () { ... }).pipe(Effect.provide(outerThing))`. Merely referencing an outer value does not require this exception: prefer a wrapper transformation when it can capture that value directly.

## Layer Naming

- When a module's primary layer represents its core service, export it as `layer`.
- Include the service or role only when the module exports multiple layers and `layer` alone would not identify what each provides.
- Add implementation or backend qualifiers after the service or role.
- When a layer is constructed from a particular input source, append `From<InputSource>`.

```ts
// Module's core service
layer
layerFromConfig
layerFromEnv

// Multiple services or roles in one module
layerFoo
layerFooFromConfig
layerFooFromEnv

layerStoreRedis
layerStoreRedisFromConfig
```

General forms:

```text
layer[FromInputSource]
layer<ServiceOrRole>[Implementation][FromInputSource]
```

Avoid `XLive`, `XLayer`, `layerFooConfig`, and `layerFooEnv`.

## Configuration

Keep configuration minimal and deliberate: verify defaults, remove duplication, and avoid speculative abstractions.
