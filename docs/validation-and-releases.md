# Validation, compatibility, and releases

Le Rust separates deterministic offline verification from authenticated provider compatibility checks. Pull requests never receive Mistral credentials.

## Offline verification

The normal suite uses an in-process HTTP server and synthetic wire fixtures. It verifies:

- exact methods, paths, query encoding, authorization, and native request bodies;
- model, chat, tool-call, structured-output, prediction, and FIM serialization;
- forward-compatible response fields and unknown finish reasons;
- SSE splitting, Unicode, CR/LF/CRLF, multiline data, queue/frame limits, `[DONE]`, and premature EOF;
- no automatic retry, disabled cross-origin redirects, timeout classification, and non-SSE rejection;
- bounded complete/error bodies and redacted default diagnostics;
- public examples, doctests, all features, MSRV, and packaged-crate compilation.

Run the contributor gate with:

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
cargo package
```

CI runs the test suite on Ubuntu 24.04, macOS 15, and current Windows, and separately checks Rust 1.88. The package job builds Cargo's extracted archive so repository-only files or path assumptions cannot hide a packaging problem.

## Live compatibility

The `Live Mistral compatibility` workflow is manually dispatched with account-available chat and FIM model IDs. It runs model discovery/retrieval, complete and streamed chat, function calling, structured output, and complete and streamed FIM. The workflow uses a protected GitHub environment and the `MISTRAL_API_KEY` secret.

Live checks verify wire compatibility, not the wording or deterministic quality of a model answer. They use caller-selected models because availability and feature support vary by account and over time. A release without a completed live run must state that limitation; mock-server tests are not evidence that the hosted service accepted the final candidate.

## API drift

`spec/source.json` pins the official OpenAPI URL and reviewed SHA-256 digest. `scripts/check-openapi.py` downloads the current source and compares it with the pin. The weekly/manual workflow fails on a changed digest so maintainers can review supported operations and update types, fixtures, and the pin together.

Drift detection never regenerates code, commits changes, or publishes a crate automatically.

## Release checklist

- [ ] Formatting, Clippy, tests, docs, MSRV, platform matrix, and package jobs pass on the release commit.
- [ ] The package archive contains the intended sources, docs, examples, and dual licenses only.
- [ ] `cargo publish --dry-run` succeeds for the exact version.
- [ ] The supported-operation inventory and OpenAPI digest are current.
- [ ] A dated live compatibility run is recorded, or its absence is explicit in the release notes.
- [ ] `CHANGELOG.md` contains the release date and user-visible compatibility notes.
- [ ] The release commit and tag are reviewed before registry publication.
- [ ] After publication, installation and docs.rs rendering are checked from the released artifact.

Released crate versions are immutable. Le Rust follows semantic versioning; before 1.0, public breaking changes increment the minor version and include migration notes. The MSRV remains stable within a minor line unless a documented security or dependency requirement forces a change.
