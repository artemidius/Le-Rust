# Validation, compatibility, and releases

Status: **planned checks, 13 November 2026**. No checks listed here have been run against an implemented Le Rust SDK. This document defines the evidence required during implementation.

## 1. Test layers

Use ordinary Rust tests for pure logic, an in-process/mock HTTP server for transport contracts, compile/doc tests for public ergonomics, and a separate manually invoked suite for real Mistral compatibility. All normal contributor tests must work without API keys or paid inference. Install dependencies in a separate setup phase when an offline test job needs them cached.

Do not assert exact model prose or deterministic fresh generations. SDK tests verify serialization, framing, state, metadata, errors, and bounds. Live tests check API compatibility and the declared response structure.

| Area | Required fixtures and assertions | First PR |
| --- | --- | --- |
| Configuration | Missing key, invalid URL, HTTPS policy, base-path joining, encoded model IDs, redacted config | P02 |
| Transport | Exact verbs/paths/auth/content types; no implicit requests; pool reuse smoke test | P02 |
| Errors | 400/401/403/404/422/429/5xx, JSON error objects, validation arrays, HTML/text errors, malformed JSON | P02 |
| Bounds | Huge body with/without Content-Length; oversized decoded content; truncated error-body flag | P02 |
| Redirects | Redirect to a second mock origin; second origin receives no request/key | P02 |
| Models | List envelope, base and fine-tuned models, unknown fields/types/capabilities, query encoding | P03 |
| Chat | Omission/null behavior; roles; null tool-call assistant content; multiple choices; structured blocks | P04 |
| Response evolution | Unknown finish reasons/fields; missing optional usage; invalid required field types | P04 |
| SSE | All representative byte splits; one-byte chunks; CR/LF/CRLF; BOM; comments; multiline data | P05 |
| Stream completion | DONE, incomplete EOF, error after chunks, finish reason before usage, DONE without TCP close | P05 |
| Cancellation | Drop future/owned stream; no background producer; no second request | P05 |
| Tools | String/object arguments; complete calls and fragments; interleaved same-name calls; conflicting IDs | P06 |
| Collection | Multiple choices/messages; distinct indices; limits; usage snapshots; incomplete collection fails | P06 |
| Structured output | Exact JSON/schema wire keys; explicit output parse errors; no inferred schema-validation claim | P06 |
| Prediction | Present/omitted native chat field; no field leaked into FIM; limits and usage preserved | P06 |
| FIM | Prefix/suffix including empty values; native endpoint; complete/stream envelope; no content rewriting | P07 |
| Resilience | Delayed headers, stalled reads, overall deadline, non-SSE 200, malformed events, concurrent isolation | P08 |
| Privacy | Sentinel keys/prompts/tool arguments/server echoes absent from Debug, Display, tracing and source-chain formatting | P02/P08 |
| Public API | All examples/doc tests; independent downstream crate; feature/MSRV combinations | P01/P09/P11 |
| Maintenance | Synthetic upstream change produces useful scoped drift report | P10 |
| Packaging | Archive contents and independently built extracted crate | P12 |

Some fixtures are intentionally invalid or more fragmented than currently observed service output. Label their purpose. Do not use synthetic tests as evidence that the service emits a particular undocumented shape.

## 2. Streaming test strategy

For every small reference stream, test every two-part split position, one-byte chunk delivery, and reproducibly randomized multi-part chunking. Include a multibyte character in ordinary content and tool arguments. Property tests should establish that valid chunk partitioning does not change decoded events or terminal state.

Test pending line and event bounds before an event terminates; test total collection and per-tool argument limits separately. Include many small events in one incoming chunk so a parser cannot evade its queue bound by framing them all at once.

Use a low-level local server when the normal mock framework cannot reproduce half-closed sockets, unusual framing, or stalled bodies. Use paused/injected time for deterministic policy tests and short bounded real I/O deadlines where virtual time cannot model the socket. No minute-long sleeps in CI.

Assert one terminal error followed by exhaustion. A stream ending after a choice finish reason but before DONE must fail. A DONE event on a persistent connection must finish promptly. A truncated model answer with a normal protocol terminator must retain its truncation reason instead of becoming a transport error.

A small parser fuzz target is useful before release and after parser changes: arbitrary bytes must never panic or grow memory beyond configured bounds. Keep regular CI runs bounded; retain any discovered case as a deterministic regression fixture.

## 3. No-retry and cancellation evidence

The mock server counts generation requests. Connection failures, 429s, 5xxs, malformed responses, and partial output must never cause another generation attempt. Review both SDK and HTTP-client retry configuration.

For cancellation, verify local ownership and cleanup: dropping the stream stops SDK polling, releases held resources, and does not leave an SDK producer or retry task. Do not assert that the remote model stopped or that billing was refunded. Include an example showing that dropping a `next()` future alone does not drop its stream.

## 4. Live compatibility workflow

Live tests are opt-in and separately gated from ordinary pull-request jobs. Use a maintainer-controlled environment, caller-selected account-available models, low output ceilings, serial execution, and explicit spending limits in the test runner. Never expose keys to untrusted contribution workflows. Models supporting one feature need not support every other tested feature.

The release report records date/time, crate commit, pinned source hash, endpoint, requested and returned model identifiers, enabled feature, result, usage when returned, and known limitations. Do not save credentials or private prompts. Captured minimal fixtures must be reviewed before being committed.

Required live cases:

1. List models and retrieve a returned model; verify actual response envelope and capabilities.
2. Complete a short chat request and observe native response/usage structure.
3. Stream chat through the documented terminator; preserve complete/partial status correctly.
4. Request a harmless function call, inspect its arguments, and submit an application-constructed matching tool result. No external side effect is necessary.
5. Exercise streamed tool output and record whether it arrives complete or fragmented; do not force the service to match a synthetic assumption.
6. Request JSON output and a small custom schema separately; parse the returned data explicitly.
7. Exercise the supported predicted-content parameter with an eligible model; record compatibility without promising savings.
8. Complete FIM with a small code prefix/suffix.
9. Stream FIM through completion and inspect its native envelope and usage.

Run list/retrieve/chat/FIM probes as soon as the corresponding PR is ready. Release-time reruns validate the final candidate. Resolve schema contradictions by combining actual observations with primary documentation; a single observation is not a permanent universal contract. Keep compatibility patches narrow and justified.

If credentials or eligible models are unavailable, record that the live gate remains open. Offline correctness can still be completed, but do not label the candidate as verified against the hosted service. This planning task makes no live calls and does not request or publish credentials.

## 5. CI and compiler matrix

At P01, choose the exact dependency set and MSRV, declare `rust-version`, and document the tested Rust edition. Do not infer compatibility from edition support alone. CI should include:

- Linux on declared MSRV and current stable: library, integration, examples and documentation tests.
- macOS and Windows current stable: compile plus portable unit/transport tests; document any test-specific exclusions.
- Default features, tracing enabled, and each documented non-default TLS/feature configuration. Do not advertise untested combinations.
- Formatting and Clippy; docs built with warnings treated as errors for project documentation.
- Dependency/license review and a separately triaged vulnerability advisory check.
- Versioned package build and independent consumer check before release.

Use lockfiles for reproducible repository/CI builds. Also test a consumer resolving the published dependency ranges: downstream library users do not inherit the SDK repository's lockfile. Do not equate an all-features build with coverage of every intended configuration.

Candidate commands, to be adjusted to the actual manifest in P01:

```text
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo test --doc
cargo doc --no-deps
cargo package --list
cargo publish --dry-run
```

Run the relevant subset per PR; run the full advertised matrix on the release candidate. Verify the extracted package rather than relying only on builds in the repository checkout.

## 6. Specification and fixture provenance

Create a manifest containing the official source URL, retrieval timestamp, SHA-256, operations in scope, and source revision where available. Vendor the selected source or a reproducibly derived subset during P00/P01; record derivation if using a subset. Normal tests consume the pin and never fetch mutable upstream schemas.

Maintain a separate overrides document for contradictions. Each exception has the source location, observed discrepancy, selected behavior, evidence, regression fixture, and review trigger. Do not silently edit the vendored upstream specification to make generation pass.

The drift report should examine covered operations and their transitive schema references. A new unrelated administrative endpoint is informational; removal or changed requiredness on a supported operation is review-worthy. Do not auto-publish or silently regenerate a handwritten public interface.

Fixtures are classified as synthetic, derived from primary SDK examples, or sanitized live observations. Avoid importing large third-party fixture collections; retain attribution and applicable notices when using external material. The source hash observed during this planning session is in [References](references.md); it is not a claim that a snapshot has already been vendored here.

## 7. Release and version policy

Use semantic versioning with an explicit pre-1.0 compatibility convention: compatible fixes preserve the `0.y` line; breaking public API or supported-behavior changes move to a new `0.y` with migration notes. Plan to keep MSRV stable within a `0.y` line; an unavoidable exception requires an explicit compatibility decision, not a silent dependency update. See the [Cargo references](references.md).

Responses remain forward-tolerant, but semantic changes in collection, retry, limits, and diagnostics still require review. A new enum variant is not automatically harmless to downstream exhaustive matches; make extensibility decisions before release.

The v0.1 release checklist:

- [ ] Name availability, repository identity, maintainership and registry ownership are resolved.
- [ ] Coverage inventory matches implemented methods, fields, content types and feature limits.
- [ ] All required offline tests, examples, docs and independent consumer checks pass.
- [ ] Live compatibility report covers the six methods and supported feature workflows on the final candidate.
- [ ] Public API, error, cancellation, no-retry and stream-terminal contracts have been reviewed.
- [ ] MSRV, targets, features and dependency policy are documented and tested.
- [ ] README clearly says unofficial and documents supported scope without unmeasured claims.
- [ ] Changelog, license/notices, examples, metadata and support instructions are complete.
- [ ] Archive contains required source/docs/licenses and excludes credentials, private fixtures and unnecessary research material.
- [ ] Package builds as an independent dependency; publish dry run passes.
- [ ] Maintainer reviews the exact release commit, version and publication action.
- [ ] After an authorized publish, registry installation and docs rendering are verified from the released artifact.

This plan does not publish anything. A tagged release and registry publication will be deliberate implementation-stage actions. A package version cannot simply be overwritten, so validate the exact archive first.

## 8. Handoff and maintenance

Le Harness adopts a tagged/published version and imports public API only. Keep the external consumer fixture small enough to run in Le Rust CI; it should not become a copy of the agent. SDK fixes found by the harness receive regression fixtures and an appropriate release.

After v0.1, triage failures into provider contract drift, SDK bug, unsupported feature, or application policy. Maintain a supported-operation report and migration notes. Choose a regular manual or scheduled drift-review cadence when implementation begins; no recurring automation is created by this planning task.

A 1.0 proposal should include evidence from real downstream use and stability of the streaming, errors, configuration and extension contracts. Add endpoint families when they have concrete demand and independent validation, not solely to increase the coverage count.
