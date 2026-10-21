import { randomUUID } from "node:crypto"

import { OpenAiClient, OpenAiLanguageModel } from "@effect/ai-openai-compat"
import { TypeSafeClient, TypeSafeDecisionModel } from "@effect/ai-typesafe"
import { Config, Layer, Option, Schema as S } from "effect"
import { HttpClient, HttpClientRequest } from "effect/http"
import type { TraitsConfig } from "traits"
import PackageJson from "traits/package.json" with { type: "json" }

const sessionId = randomUUID()

const layerTypeSafe = TypeSafeClient.layerConfig({ apiKey: Config.Redacted("TYPESAFE_API_KEY") })

const layerOpenCode = OpenAiClient.layerConfig({
  apiKey: Config.Redacted("OPENCODE_API_KEY"),
  apiUrl: Config.succeed("https://opencode.ai/inference/go/openai/v1"),
  transformClient: HttpClient.mapRequest((request) => {
    const withHeaders = request.pipe(
      HttpClientRequest.setHeader("x-opencode-session", sessionId),
      HttpClientRequest.setHeader("user-agent", `traits/${PackageJson.version}`)
    )
    const body = withHeaders.body
    if (body._tag !== "Uint8Array" || body.text === undefined) return withHeaders
    const payload = Option.getOrUndefined(S.decodeOption(S.fromJsonString(S.Record(S.String, S.Unknown)))(body.text))
    if (payload === undefined) return withHeaders
    const format = Option.getOrUndefined(
      S.decodeUnknownOption(
        S.Struct({
          type: S.Literal("json_schema"),
          json_schema: S.Struct({ schema: S.Unknown })
        })
      )(payload.response_format)
    )
    if (format === undefined) return withHeaders
    const messages = Option.getOrElse(
      S.decodeUnknownOption(S.Array(S.Record(S.String, S.Unknown)))(payload.messages),
      () => []
    )
    return HttpClientRequest.bodyJsonUnsafe(withHeaders, {
      ...payload,
      response_format: { type: "json_object" },
      messages: [
        ...messages,
        {
          role: "system",
          content: `Respond with only a JSON object matching this JSON Schema:\n${JSON.stringify(format.json_schema.schema)}`
        }
      ]
    })
  })
})

export default {
  excludes: ["repos/**"],
  layerDecisionModel: TypeSafeDecisionModel.layer({ model: "jev-latest" }).pipe(Layer.provide(layerTypeSafe)),
  layerLanguageModel: OpenAiLanguageModel.layer({ model: "deepseek-v4.1-flash" }).pipe(Layer.provide(layerOpenCode))
} satisfies TraitsConfig
