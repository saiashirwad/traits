// oxlint-disable effecttsgo/async-function effecttsgo/missing-pipeable-signature effecttsgo/strict-boolean-expressions
// Node runs this loader hook before the package's dependencies, including Effect, are available.

import { readFile } from "node:fs/promises"
import { stripTypeScriptTypes } from "node:module"

// stripTypeScriptTypes is experimental; keep its warning out of MCP stderr.
const emitWarning = process.emitWarning.bind(process)
process.emitWarning = (warning, ...args) => {
  if (String(warning).includes("stripTypeScriptTypes")) return
  emitWarning(warning, ...args)
}

export async function load(url, context, nextLoad) {
  if (url.endsWith(".ts")) {
    const source = await readFile(new URL(url), "utf8")
    return {
      format: "module",
      source: stripTypeScriptTypes(source, { mode: "transform" }),
      shortCircuit: true
    }
  }
  return nextLoad(url, context)
}
