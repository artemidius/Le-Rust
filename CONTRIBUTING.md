# Contributing to Le Rust

Thank you for improving Le Rust. Contributions should keep the SDK's public contract small, explicit, and testable.

## Before opening a change

For fixes, start with a failing regression fixture. For API additions, describe the native Mistral operation or field, the intended Rust interface, compatibility behavior, and why the extension belongs in the SDK rather than an application.

Avoid committing API keys, prompts from private systems, generated outputs containing private data, or unreviewed live response captures. Use synthetic fixtures unless a sanitized live fixture is necessary to resolve a protocol ambiguity.

## Local checks

Le Rust declares Rust 1.88 as its minimum supported version and also tracks current stable. Run:

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --doc --all-features
cargo package --allow-dirty
```

Normal tests do not require network access or a Mistral API key. Live checks are maintainer-controlled and never run on untrusted pull requests with credentials.

## Compatibility changes

Provider responses are intentionally forward-tolerant. Request construction is strict and explicit. Do not solve a schema discrepancy by silently normalizing every unknown input; record the discrepancy in `docs/references.md`, add a scoped fixture, and document the selected behavior.

During the `0.y` series, breaking public changes require a new `0.y` release and migration notes. Keep the MSRV stable within a release line unless a documented dependency or security requirement makes an update necessary.

## Pull requests

Keep changes focused and include tests and documentation in the same pull request. Explain the user-visible behavior, validation performed, and any live compatibility limits. All CI checks must pass before merge.
