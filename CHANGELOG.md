# Changelog

All notable changes to Le Rust are documented here.

The project follows Semantic Versioning. During the `0.y` series, a change to `y` may contain breaking public API changes.

## 0.1.1 - 2026-11-13

### Added

- Public constructors for function calls and tool calls so agent runtimes can replay native assistant tool-call messages without manufacturing response values through Serde.

## 0.1.0 - 2026-11-13

### Added

- Asynchronous clients for models, chat completions, and FIM completions.
- Incremental chat and FIM streaming with explicit protocol completion.
- Native function-tool, structured-output, and predicted-output request types.
- Bounded response handling, redacted errors, cancellation by ownership, and no automatic retries.
- Explicit typed parsing for structured output and fallible text access for chat/FIM responses.
- Forward-tolerant model-list and alternate completion-choice decoding.
- Classified OpenAPI drift reports with synthetic regression tests.
- Independent extracted-package consumer verification across every public operation.

### Compatibility

- Minimum supported Rust version: 1.88.
- Reviewed against Mistral's OpenAPI digest `7b344504336785172c86cd826ce4b1cbfd12be228a8432cf046b2b8fdafacf72` on 2026-11-13.
- Authenticated Mistral compatibility was not run for this candidate because no API credential was available; the manual workflow remains the provider gate.
