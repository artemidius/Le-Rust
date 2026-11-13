# Research evidence and primary references

Checked **13 November 2026**. Research used public documentation, the official OpenAPI source, and SDK source/API documentation. No authenticated inference request, package build, benchmark, or runtime comparison was performed. Planned behavior in the other documents is a Le Rust design decision unless identified as provider behavior.

## API sources

| Source | What it establishes |
| --- | --- |
| [Mistral SDK documentation](https://docs.mistral.ai/resources/sdks) | Official documented SDK languages and third-party ecosystem context |
| [Official OpenAPI](https://docs.mistral.ai/openapi.yaml) | Machine-readable operations and referenced schemas; requires discrepancy review |
| [Chat reference](https://docs.mistral.ai/api/endpoint/chat) | Native completion endpoint, messages, options, streaming and response family |
| [Models reference](https://docs.mistral.ai/api/endpoint/models) | Discovery/retrieval and model-card metadata |
| [FIM reference](https://docs.mistral.ai/api/endpoint/fim) | Native prefix/suffix completion and streaming interface |
| [Chat guide](https://docs.mistral.ai/inference/conversations/chat-completion) | Message and conversation usage |
| [Function calling](https://docs.mistral.ai/inference/conversations/function-calling) | Tool definitions, returned calls, result correlation and application execution |
| [Structured outputs](https://docs.mistral.ai/inference/conversations/structured-output) | JSON versus custom schema output and prompting requirements |
| [Custom schema output](https://docs.mistral.ai/inference/conversations/structured-output/custom) | Native nested schema representation |
| [Predicted outputs](https://docs.mistral.ai/inference/conversations/advanced/predicted-outputs) | Separate prediction parameter; compatibility is model-dependent |
| [Error glossary](https://docs.mistral.ai/resources/error-glossary) | Error status/body families beyond endpoint success examples |

Observed OpenAPI SHA-256 on this date:

```text
7b344504336785172c86cd826ce4b1cbfd12be228a8432cf046b2b8fdafacf72
```

The hash identifies the source inspected during planning. The file has not been vendored into this documentation folder. P00/P01 must fetch, record, and pin the implementation baseline; upstream may have changed in the meantime.

Official Python SDK source was cross-checked at revision `d3da3bd3c63d0502449e1e765bc2d3c16d0d2f1d`:

- [SSE parser](https://github.com/mistralai/client-python/blob/d3da3bd3c63d0502449e1e765bc2d3c16d0d2f1d/src/mistralai/client/utils/eventstreaming.py): distinguish the SDK event wrapper from actual SSE JSON data.
- [Completion response](https://github.com/mistralai/client-python/blob/d3da3bd3c63d0502449e1e765bc2d3c16d0d2f1d/src/mistralai/client/models/chatcompletionresponse.py): cross-check the schema's required-field anomaly.
- [Usage type](https://github.com/mistralai/client-python/blob/d3da3bd3c63d0502449e1e765bc2d3c16d0d2f1d/src/mistralai/client/models/usageinfo.py): compare omitted/additional usage handling.
- [Retry implementation](https://github.com/mistralai/client-python/blob/d3da3bd3c63d0502449e1e765bc2d3c16d0d2f1d/src/mistralai/client/utils/retries.py): optional retry-header handling, not a mandate to adopt its policy.

Source implementations are corroborating evidence, not proof of every live service behavior.

## Discrepancies and investigation tasks

| Observation in inspected sources | Proposed treatment / evidence still needed |
| --- | --- |
| Model-list schema has an envelope; examples include a bare array | Use schema and official client evidence; verify with list-models live probe |
| Model discriminator casing differs between schema and some examples | Preserve variants carefully; do not add broad normalization without observed need |
| Chat response required list mentions an undeclared `data` field | Do not blindly generate from this inconsistency; record scoped resolution against SDK and live response |
| FIM schema combines required model with an old default | Require caller-selected model; do not inherit that default |
| Full ToolCall types are reused for stream data | Keep distinct delta types; preserve whole calls and explicitly test possible fragments |
| Tool arguments permit string or object representation | Preserve both; no blind string concatenation or double encoding |
| Complete and streamed finish-reason enums differ | Preserve unknown strings and record actual streamed values |
| Provider usage requiredness/extra-field handling differs from SDK models | Keep unknown/missing usage explicit; verify live observations |
| Normal errors and 422 validation errors have different shapes | Decode known families; keep bounded unknown bodies |

Live tool-argument fragmentation, header availability, precise usage details, and feature support for a given account/model remain unverified. Do not claim OpenAI transport similarity proves matching Mistral semantics. A local custom endpoint is an extension point, not a supported provider declaration.

## Rust and protocol references

- [reqwest client configuration](https://docs.rs/reqwest/latest/reqwest/struct.ClientBuilder.html): timeouts, redirects, and transport options. Select and pin a tested version in P01.
- [reqwest response streaming](https://docs.rs/reqwest/latest/reqwest/struct.Response.html): byte streams are not parsed SSE events.
- [Disable reqwest retries](https://docs.rs/reqwest/latest/reqwest/retry/fn.never.html): default protocol retries must be disabled for Le Rust's no-retry contract.
- [WHATWG SSE parsing](https://html.spec.whatwg.org/multipage/server-sent-events.html#parsing-an-event-stream): event framing; browser reconnection behavior is not adopted for generation POSTs.
- [Tokio JoinHandle](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html): dropping a task handle detaches it; prefer no SDK-owned producer task.
- [Tokio timeout](https://docs.rs/tokio/latest/tokio/time/fn.timeout.html): future/deadline ownership semantics.
- [Cargo MSRV](https://doc.rust-lang.org/cargo/reference/rust-version.html): declaring and testing the compiler floor.
- [Cargo SemVer](https://doc.rust-lang.org/cargo/reference/semver.html): public API compatibility and pre-1.0 boundaries.
- [Cargo publishing](https://doc.rust-lang.org/cargo/reference/publishing.html): package metadata, name availability, dry runs, and immutable released versions.

## Related work and positioning

[mistralai-sdk](https://github.com/adriendellagaspera/mistralai-rs) is an existing independent Rust client. Its [published documentation](https://docs.rs/mistralai-sdk/latest/mistralai_sdk/) and [FIM API](https://docs.rs/mistralai-sdk/latest/mistralai_sdk/api/resources/fim/fim/struct.FimClient.html) already expose asynchronous and streaming functionality. Current project documentation also describes schema-based generation and maintenance. These are not vacant features that Le Rust can claim to invent.

Le Rust's chosen value is a deliberate public API and explicit operational contracts, documented through tests and a real downstream consumer. Actual superiority would require a fair runtime/API comparison; none has been performed here. Implementation review may learn from other clients without importing code or assuming their limitations.

Before implementation, verify name availability and the actual dependency/toolchain combination. Before release, verify supported endpoints with account-available models. These checks refine technical details without reopening the user's chosen Rust language, independent SDK, or Le Rust name.
