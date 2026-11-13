# Design context: Le Rust and the generated community SDK

This comparison records the decision context for Le Rust 0.1. It is deliberately narrow: it compares three workflows that matter to this crate and does not rank the projects or claim comprehensive superiority.

The comparison was refreshed on **13 November 2026** against [`mistralai-sdk` 0.5.0](https://crates.io/crates/mistralai-sdk/0.5.0), whose source identifies it as a reproducibly generated client for the official OpenAPI specification. Its published metadata declares Rust 1.94 and exposes a substantially broader Mistral API surface. The crate and its chat/FIM stream feature compiled during this review on Rust 1.100; authenticated service calls were outside this comparison.

## Representative workflows

| Workflow | `mistralai-sdk` 0.5.0 | Le Rust 0.1 design choice |
| --- | --- | --- |
| Stream chat | Provides `ChatClient::complete_stream` and typed stream items | Owns a small SSE state machine with bounded frames, `[DONE]` as the only successful protocol terminator, premature-EOF errors, direct caller backpressure, and an optional bounded collector |
| Tool round trip | Generated chat types carry tool definitions, returned tool calls, and tool-result messages | Keeps tool execution in the application; preserves string/object argument forms; provides fallible argument parsing and index-aware bounded accumulation for fragmented calls |
| FIM complete and stream | Provides `FimClient::complete` and `FimClient::complete_stream` | Uses distinct handwritten `FimRequest` builders over the same bounded transport and stream machinery; requires an explicit model and offers a fallible text accessor |

FIM availability is therefore not a unique Le Rust capability. The differentiator is the deliberately small, handwritten contract around these workflows: six public operations, Rust 1.88 support, no automatic generation retries, explicit limits, redacted diagnostics, and extracted-package consumer tests. The generated SDK is the better fit when broad platform coverage is the primary requirement.

## Boundary decision

Le Rust is an SDK, not an agent runtime. It exposes native responses, stream deltas, usage, request metadata, and explicit parsers. A consumer owns model routing, prompt budgets, tool authorization and execution, retry decisions, filesystem writes, and edit policy. Keeping that boundary makes the crate independently useful and prevents an application-specific orchestration loop from becoming part of the wire client.

Sources: the community crate's [published metadata](https://crates.io/crates/mistralai-sdk/0.5.0), [repository](https://github.com/adriendellagaspera/mistralai-rs), and generated [API documentation](https://docs.rs/mistralai-sdk/0.5.0/mistralai_sdk/).
