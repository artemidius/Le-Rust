# Le Rust

[![CI](https://github.com/artemidius/Le-Rust/actions/workflows/ci.yml/badge.svg)](https://github.com/artemidius/Le-Rust/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

Le Rust is an unofficial asynchronous Rust SDK for the native [Mistral AI API](https://docs.mistral.ai/api/). It provides a compact, typed interface for model discovery, chat completions, function-tool payloads, structured outputs, predicted outputs, and Codestral fill-in-the-middle completion.

The crate keeps transport behavior explicit: generation requests are never retried automatically, streamed results are complete only after Mistral's terminal marker, response buffers are bounded, and provider errors retain diagnostic details behind an explicit accessor.

Le Rust is independent software and is not affiliated with or endorsed by Mistral AI.

## Supported API

| Resource | Complete | Stream |
| --- | --- | --- |
| Models: list and retrieve | Yes | Not applicable |
| Chat completions | Yes | Yes |
| FIM completions | Yes | Yes |
| Function-tool payloads | Yes | Yes, with explicit collection |
| JSON and JSON Schema response formats | Yes | Native deltas |
| Predicted output | Yes | Native deltas |

Model identifiers are caller-supplied strings. Le Rust never selects a model, executes a tool, modifies a prompt, or applies generated code to a file.

## Installation

```toml
[dependencies]
le-rust = { git = "https://github.com/artemidius/Le-Rust", tag = "v0.1.2" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The crates.io package is prepared but not yet published. Use the release tag (or pin its commit) for reproducible builds until the first registry release.

Set `MISTRAL_API_KEY` in the process environment, then create a client explicitly:

```rust,no_run
use le_rust::chat::{ChatRequest, Message};
use le_rust::{Client, RequestOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    let model = std::env::var("MISTRAL_MODEL")?;
    let response = client
        .chat()
        .complete(
            ChatRequest::new(model, vec![Message::user("Explain ownership in one sentence.")]),
            RequestOptions::default(),
        )
        .await?;

    if let Some(text) = response
        .data
        .choices
        .first()
        .and_then(|choice| choice.message())
        .and_then(|message| message.text())
    {
        println!("{text}");
    }
    Ok(())
}
```

`Client::builder()` exposes explicit credentials, base URL, timeouts, and byte limits. Plain HTTP custom endpoints require an opt-in and are accepted only on loopback addresses.

## Streaming

`chat().stream(...)` and `fim().stream(...)` return a directly polled stream of native completion chunks. Receiving a choice finish reason does not finish the transport: the stream continues until `data: [DONE]`, allowing later usage or additional-choice events to arrive.

```rust,no_run
use futures_util::StreamExt;
use le_rust::chat::{ChatRequest, Content, Message};
use le_rust::{Client, RequestOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    let model = std::env::var("MISTRAL_MODEL")?;
    let mut stream = client
        .chat()
        .stream(
            ChatRequest::new(model, vec![Message::user("Write one line of Rust.")]),
            RequestOptions::default(),
        )
        .await?;

    while let Some(chunk) = stream.next().await {
        for choice in chunk?.choices {
            if let Some(Content::Text(text)) = choice.delta.content {
                print!("{text}");
            }
        }
    }
    Ok(())
}
```

Dropping the owned stream stops local consumption. It cannot guarantee that remote inference stops or that billing is avoided. A partial disconnect is returned as an error and is never combined with a retried generation.

## Tools and structured output

Le Rust serializes function definitions, preserves returned call identifiers, accepts native string or object arguments, and provides a fallible typed argument parser. Applications remain responsible for schema validation, authorization, and execution.

Use `ResponseFormat::JsonSchema` to send a native JSON Schema response format. Successful JSON decoding confirms syntax and the requested Rust shape; it does not replace application validation.

Runnable examples cover [models](examples/models.rs), [chat](examples/chat.rs), [streaming](examples/stream.rs), [function tools](examples/tools.rs), [structured output](examples/structured_output.rs), and [FIM](examples/fim.rs).

## Reliability contract

- No automatic retries, including on 429, 5xx, connection failures, or protocol NACKs.
- Redirects are disabled so bearer credentials cannot be forwarded to another origin.
- Complete bodies, error bodies, pending SSE frames, collected output, and tool arguments are bounded.
- Unknown response fields and finish reasons are preserved where possible.
- Client and error `Debug`/`Display`, plus optional tracing, omit credentials and request/response content.
- Provider error bodies are available only through `ApiError::diagnostic_body()` and may contain submitted content.

See the [API and runtime contract](docs/api-and-runtime-contract.md), [design context](docs/design-comparison.md), and [validation policy](docs/validation-and-releases.md) for details.

## Development

The minimum supported Rust version is 1.88. Normal tests use local HTTP fixtures and require no API key:

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --doc --all-features
```

Live compatibility checks are opt-in and require caller-selected models. They test wire compatibility rather than model-answer determinism.

## License

Licensed under either the [Apache License, Version 2.0](LICENSE-APACHE) or the [MIT license](LICENSE-MIT), at your option.
