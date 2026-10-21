#!/usr/bin/env node
import { register } from "node:module"

register(new URL("./ts-loader.js", import.meta.url))
await import("./bin.ts")
