# Architecture and implementation notes

Le Rust is a small asynchronous SDK for the native Mistral API. Its first release deliberately covers model discovery, chat completions, function-tool payloads, structured and predicted output, and FIM completions. It is not a generated client for the entire platform.

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

Requests expose typed fields for the supported core and a collision-checked extension map for newer chat fields. Responses preserve unknown data instead of rejecting it. The official OpenAPI source is pinned by digest in `spec/source.json`; the scheduled drift check reports a changed digest but never rewrites public Rust types or publishes a release.

The minimum supported compiler is Rust 1.88. The default TLS backend is rustls. `tracing` is optional and emits metadata only; the crate never installs a subscriber.

## Deferred scope

Embeddings, OCR, files, batches, audio/realtime, hosted agents, fine-tuning administration, service-account token rotation, automatic retries, blocking APIs, and WASM support are outside 0.1. New endpoint families should include a typed contract, wire fixtures, examples, and explicit compatibility behavior rather than expanding the crate only for endpoint count.
