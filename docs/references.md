# Protocol sources and compatibility notes

The 0.1 implementation was checked against Mistral's primary documentation and official OpenAPI source on **13 November 2026**.

## Primary sources

| Source | Used for |
| --- | --- |
| [Official OpenAPI](https://docs.mistral.ai/openapi.yaml) | Operation paths, request fields, response families, and schema drift pin |
| [Chat API](https://docs.mistral.ai/api/endpoint/chat) | Native chat request, response, streaming, tools, prediction, and output formats |
| [Models API](https://docs.mistral.ai/api/endpoint/models) | Model discovery, retrieval, cards, and capabilities |
| [FIM API](https://docs.mistral.ai/api/endpoint/fim) | Prefix/suffix completion and native streaming |
| [Function calling](https://docs.mistral.ai/inference/conversations/function-calling) | Tool definitions, call IDs, arguments, and result correlation |
| [Structured outputs](https://docs.mistral.ai/inference/conversations/structured-output) | JSON and JSON Schema behavior and caller responsibilities |
| [Error glossary](https://docs.mistral.ai/resources/error-glossary) | Provider error families |
| [Official Python SDK](https://github.com/mistralai/client-python) | Corroborating generated types where examples and schema presentation differ |

Reviewed OpenAPI SHA-256:

```text
7b344504336785172c86cd826ce4b1cbfd12be228a8432cf046b2b8fdafacf72
```

The digest and source URL are machine-readable in `spec/source.json` and checked by the scheduled drift workflow.

## Deliberate compatibility choices

| Source ambiguity or evolution risk | Le Rust behavior |
| --- | --- |
| Model-list examples and schema presentation have differed | Decode the documented list envelope and preserve additional fields; verify through the manual live workflow |
| FIM documentation historically showed a default model | Require an explicit caller-selected model |
| Complete tool calls and streamed tool-call fragments share related schemas | Keep complete and delta Rust types separate; collect fragments by explicit indices |
| Tool arguments may be a string or JSON object | Preserve both and parse only through an explicit fallible helper |
| Finish-reason values can evolve | Keep known variants plus `Unknown(String)` |
| Usage and capability fields can be missing or grow | Use optional known values and preserve extras |
| Error bodies may be JSON, validation arrays, text, or HTML | Preserve status/metadata and a bounded raw body without exposing it in default formatting |

OpenAPI and official SDK source are compatibility evidence, not a substitute for authenticated tests against account-available models. Synthetic stream fragmentation tests are intentionally more adversarial than ordinary service output.

## Runtime references

- [reqwest client configuration](https://docs.rs/reqwest/latest/reqwest/struct.ClientBuilder.html)
- [reqwest response streaming](https://docs.rs/reqwest/latest/reqwest/struct.Response.html)
- [reqwest retry policy](https://docs.rs/reqwest/latest/reqwest/retry/fn.never.html)
- [WHATWG event-stream parsing](https://html.spec.whatwg.org/multipage/server-sent-events.html#parsing-an-event-stream)
- [Cargo `rust-version`](https://doc.rust-lang.org/cargo/reference/rust-version.html)
- [Cargo semantic versioning](https://doc.rust-lang.org/cargo/reference/semver.html)
