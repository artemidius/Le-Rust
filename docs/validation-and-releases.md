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

`spec/source.json` pins the official OpenAPI URL and reviewed SHA-256 digest. `spec/covered-openapi.json` stores a canonical snapshot of supported operations plus their transitive schemas. `scripts/check-openapi.py` classifies the current source as an exact match, an unrelated upstream change, or a covered-surface change. Unrelated endpoint additions produce a report without failing; a covered change identifies affected operations and references and fails for review.

`python3 -m unittest scripts/test_check_openapi.py` proves both classifications using synthetic schemas. The weekly/manual workflow always uploads the JSON report. Drift detection never regenerates code, commits changes, or publishes a crate automatically.

## Extracted-package consumer

`scripts/test-package-consumer.sh` runs `cargo package`, extracts the resulting archive into a temporary directory, and builds a separate consumer whose dependency points only at that extracted package. Its local mock exercises all six methods plus chat streaming, fragmented tool-call collection, explicit tool-result continuation, JSON Schema parsing, and FIM streaming. This catches missing package files, private-module dependencies, and workspace-only path assumptions.

## Release checklist

- [x] Formatting, Clippy, tests, docs, MSRV, platform matrix, and package jobs pass on the release commit.
- [x] The package archive contains the intended sources, docs, examples, and dual licenses only.
- [x] `cargo publish --dry-run` succeeds for the exact version.
- [x] The supported-operation inventory and OpenAPI digest are current.
- [x] A dated live compatibility run is recorded, or its absence is explicit in the release notes.
- [x] `CHANGELOG.md` contains the release date and user-visible compatibility notes.
- [x] The release commit and tag are reviewed before registry publication.
- [ ] After publication, installation and docs.rs rendering are checked from the released artifact.

Released crate versions are immutable. Le Rust follows semantic versioning; before 1.0, public breaking changes increment the minor version and include migration notes. The MSRV remains stable within a minor line unless a documented security or dependency requirement forces a change.
