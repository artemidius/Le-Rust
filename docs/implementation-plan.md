# Architecture and implementation notes

Le Rust is a small asynchronous SDK for the native Mistral API. Its first release deliberately covers model discovery, chat completions, function-tool payloads, structured and predicted output, and FIM completions. It is not a generated client for the entire platform.

## Implementation status

The original 0.1 backlog is complete in the release candidate. Evidence is kept with the behavior rather than as an unchecked promise.

| Phase | Result | Primary evidence |
| --- | --- | --- |
| P00 — contract and provenance | Complete | [`spec/coverage.toml`](../spec/coverage.toml), [`spec/source.json`](../spec/source.json), [design comparison](design-comparison.md) |
| P01 — crate foundation | Complete | dual licenses, CI, Rust 1.88 job, examples, contributor and security policies |
| P02 — transport and errors | Complete | client/transport unit and mock-server tests; disabled redirects/retries; bounded redacted errors |
| P03 — models | Complete | list/retrieve contract tests, encoded IDs, forward-compatible cards, envelope/bare-list decoding |
| P04 — complete chat | Complete | typed request builders, replayable responses, alternate native choice shapes, forward-compatible fields |
| P05 — SSE | Complete | exhaustive split tests, one-byte/Unicode/line-ending fixtures, limits, explicit terminal state |
| P06 — tools and output | Complete | indexed bounded collector, tool continuation example, response formats, typed JSON parser, prediction |
| P07 — FIM | Complete | native complete/stream methods, fallible text accessor, examples and disconnect/limit tests |
| P08 — diagnostics | Complete | timing, non-SSE, malformed-body, concurrency, redaction, retry metadata, optional tracing tests |
| P09 — documentation | Complete | README, examples, API/runtime contract, references, validation and release policy |
| P10 — API drift | Complete | covered-surface snapshot, classified report, synthetic change tests, weekly/manual artifact workflow |
| P11 — independent consumer | Complete | extracted `.crate` consumer exercises discovery, streaming tools, continuation, JSON, and FIM |
| P12 — release candidate | Complete | [`release-evidence/0.1.0.md`](release-evidence/0.1.0.md), package/dry-run gates, reviewed tag and release |

Registry publication is a separate maintainer action. Authenticated provider compatibility is also separately recorded because deterministic mocks cannot substitute for an account-backed call.

## Design goals

- Preserve Mistral-native request and response semantics instead of hiding them behind a provider-neutral interface.
- Make network behavior observable and predictable: one SDK call makes one HTTP request, redirects and retries are disabled, and timeouts are classified.
- Stream without a background producer. The caller's polling provides backpressure and dropping the owned stream stops local consumption.
- Bound every provider-controlled buffer used by the transport or collector.
- Remain forward-tolerant by preserving unknown response fields, content blocks, usage fields, and finish reasons.
- Keep application policy outside the SDK. Model routing, prompt construction, tool execution, filesystem changes, and retry decisions belong to the caller.

## Data path

```text
request builder
    -> structural validation
    -> reqwest transport
    -> HTTP status and metadata
    -> bounded JSON decoder
       or incremental SSE decoder
    -> typed native response/delta
    -> optional bounded stream collector
```

`Client` owns a shared `reqwest::Client` and is cheaply cloneable. Resource accessors borrow the client and add no state. Complete calls decode a bounded response body. Streaming calls return `CompletionStream`, which directly polls the response body and yields native `CompletionChunk` values.

## Module ownership

| Module | Responsibility |
| --- | --- |
| `client` | Configuration, authentication, URL construction, transport, deadlines, body limits, and response metadata |
| `models` | Model list/retrieve requests and forward-compatible model cards |
| `chat` | Chat requests, role-specific messages, tools, response formats, complete responses, and stream deltas |
| `fim` | Native FIM request construction over the shared completion transport |
| `stream` | SSE framing, terminal-state handling, and explicit bounded collection |
| `error` | Stable error categories and redacted provider diagnostics |

## Streaming state contract

The stream has three meaningful states:

1. Open: bytes and decoded chunks may still arrive.
2. Protocol-complete: `data: [DONE]` was observed; the stream ends immediately without waiting for socket closure.
3. Terminal failure: framing, decoding, transport, limit, or premature-EOF failure was observed; exactly one error is yielded and later polls return `None`.

A choice finish reason is model output, not the transport terminator. The decoder continues through later usage and choice events until `[DONE]`. EOF before that marker is `UnexpectedEof`, even when one or more chunks were already delivered.

## Compatibility policy

Requests expose typed fields for the supported core and a collision-checked extension map for newer chat fields. Responses preserve unknown data instead of rejecting it. The official OpenAPI source is pinned by digest in `spec/source.json`. The drift check compares a canonical snapshot of only supported operations and transitive schemas: unrelated upstream additions are reported without failing, while changes affecting covered surfaces fail with their operations and references identified. It never rewrites public Rust types or publishes a release.

The minimum supported compiler is Rust 1.88. The default TLS backend is rustls. `tracing` is optional and emits metadata only; the crate never installs a subscriber.

## Deferred scope

Embeddings, OCR, files, batches, audio/realtime, hosted agents, fine-tuning administration, service-account token rotation, automatic retries, blocking APIs, and WASM support are outside 0.1. New endpoint families should include a typed contract, wire fixtures, examples, and explicit compatibility behavior rather than expanding the crate only for endpoint count.
