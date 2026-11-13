# Le Rust implementation plan

Date: **13 November 2026**. Status: **proposed implementation baseline**.

This plan translates the user's chosen standalone SDK and Rust requirement into reviewable work. It does not authorize publication or represent implemented functionality. Product name and language are settled; the technical defaults below are recommendations to use unless implementation evidence requires a documented change.

## 1. Objective and release outcome

Build an unofficial Rust client that a developer can install, configure, and use for native Mistral model discovery, chat, tool-call exchanges, structured outputs, and Codestral FIM. Provide predictable behavior when responses arrive incrementally, fail midway, evolve, or exceed limits.

The first release is successful when an independent consumer can use all six supported methods through the public API, errors remain actionable without exposing content by default, streaming cannot silently return incomplete output as success, and the published package is supported by offline tests plus dated live compatibility evidence.

Le Harness is the first consumer, but the SDK must have no dependency on harness concepts. It must be useful without a terminal UI, project directory, agent framework, or tool runner.

## 2. Chosen implementation defaults

| Decision | Baseline |
| --- | --- |
| Language and packaging | Rust, one library crate with examples; no SDK executable required |
| Package identity | Intended `le-rust` / Rust import `le_rust`; verify availability before release |
| Runtime | Async Tokio; no blocking facade or internal runtime creation in v0.1 |
| Transport | Shared reqwest client; explicitly configured Rustls TLS |
| Serialization | Serde / serde_json; separate complete-response and stream-delta types |
| API design | Handwritten public builders and methods; pinned schema and fixtures check compatibility |
| Dependencies | Select compatible published versions in P01; lock CI builds and test consumers separately |
| Compiler support | Rust 2024 edition, exact MSRV established by compiling the chosen dependency set in P01 |
| License | Proposed MIT OR Apache-2.0, with license and attribution files added during foundation work |
| Features | Native TLS selection kept narrow; optional metadata-only tracing; no per-endpoint feature maze |
| Retries | Zero automatic retries in v0.1, including transport-level retries |
| Default platform | Native Rust on Linux, macOS, Windows; no WASM/no_std promise |
| Distribution | Source crate and documentation; Le Harness consumes a tagged/published version |

The exact dependency versions and compiler floor are deliberately not guessed here. P01 must record and test them. Tokio, reqwest, Serde, an error-derivation crate, and a stream-trait crate should cover most runtime needs. Evaluate an existing SSE parser against the acceptance suite before adding a small internal parser; do not build an HTTP stack or general SDK generator.

## 3. Scope and coverage contract

| API surface | v0.1 commitment | Explicit limit |
| --- | --- | --- |
| `GET /v1/models` | Model cards, capabilities, aliases, context metadata; declared query options | No hidden discovery request before inference |
| `GET /v1/models/{model_id}` | Retrieve a caller-selected opaque model ID | No management/delete/archive methods |
| `POST /v1/chat/completions` | Complete and streaming calls; caller-owned messages | No hosted Responses/Agents/Conversations API in this release |
| Function calling | Definitions, choices, returned calls, matching tool-result messages, explicit assembly helper | SDK never executes tools or repairs their output |
| Structured output | Text, JSON object, JSON Schema request modes; explicit typed JSON parsing helper | No claim that Serde validates arbitrary JSON Schema or business correctness |
| Predicted output | Native chat request field and a small compatibility example | No automatic selection, optimization, or cost claim |
| `POST /v1/fim/completions` | Complete and streaming; prompt/suffix and documented options | No file location, patch generation policy, or file writes |
| Content | Text ergonomics; preserve returned structured blocks and metadata on history replay | Full multimodal convenience API deferred; unsupported aggregation reported explicitly |
| Accounting | Provider-reported usage and safe response metadata | No hardcoded prices, local billing reconstruction, or exact tokenizer claim |

P00/P01 will create an endpoint-and-field inventory with `typed`, `explicit extension`, `preserved on response`, and `unsupported` states. An operation is not marked fully covered merely because its endpoint can be called. Response preservation and fully understood convenience behavior are separate claims.

Deferred: embeddings, OCR, files/uploads, batch, audio/realtime, fine-tuning, administrative APIs, hosted agents, service-account refresh automation, local inference backends, automatic retry policies, and schema-derived request macros. Each later addition needs a consumer, contract, examples, and tests.

## 4. Architecture and ownership

```text
Consumer: Le Harness or another Rust application
    |
Public Client + resource methods + request builders
    |
Request validation / native wire serialization
    |
HTTP transport / limits / timeouts / metadata / errors
    |
Complete JSON decoder OR bounded SSE decoder
    |
Typed responses and deltas -> optional bounded accumulator
```

Suggested implementation layout, to be created during implementation:

```text
src/
  lib.rs             public exports and crate documentation
  client.rs          shared client and resource accessors
  config.rs          authentication, base URL, timeouts, limits
  error.rs           error categories and redacted diagnostics
  transport.rs       requests, status handling, metadata
  models.rs          discovery operations and cards
  chat/              requests, messages, responses, tool payloads
  fim.rs             FIM requests and operations
  stream/            SSE framing, deltas, terminal state, accumulator
tests/               contracts, mock HTTP, downstream consumer
examples/            compiling application-facing examples
spec/                pinned source, manifest, coverage, overrides
docs/                design, usage, support and release evidence
```

Avoid a public transport trait, plugin ABI, multiple crates, custom task scheduler, or universal model-provider abstraction until a real integration requires one. Keep the transport internally replaceable and test it through a local mock HTTP server. The SDK owns no background agent or persisted conversation.

## 5. Milestones and critical path

| Milestone | Work | Exit evidence |
| --- | --- | --- |
| M0: contract baseline | P00–P01 | Supported surface, compileable API sketch, source pin, compiler/dependency decisions |
| M1: complete inference | P02–P04 | Models and complete chat pass wire-contract tests |
| M2: usable alpha | P05–P07 | Stream termination, tools, structured output and FIM work against fixtures and early live probes |
| M3: release candidate | P08–P11 | Failure matrix, examples, maintenance checks and external consumer pass |
| M4: v0.1 | P12 | Packaged artifact, compatibility report and release checklist complete |

Critical path: `P00 -> P01 -> P02 -> P03/P04 -> P05 -> P06/P07 -> P08 -> P11 -> P12`. Documentation starts with each behavior, P09 consolidates it; P10 extends the initial schema pin into a maintenance workflow. P06 and P07 can proceed independently once P05 interfaces are stable. Run small live probes as operations become available rather than leaving every provider uncertainty until release.

Planning allowance for one experienced Rust engineer: approximately **10–15 focused engineering days**, including tests, documentation, packaging, and contract investigation. A functional alpha can precede the release candidate. This is an estimate, not a deadline; service access or protocol discrepancies can extend it. Full Mistral platform coverage is outside this estimate.

## 6. Reviewable PR backlog

Every PR includes tests for new behavior and updates to the coverage inventory. An acceptance gate is required evidence, not a claim that the work has already passed.

### P00 — Scope, public contract, and provenance

Deliver the supported-method matrix, proposed public API, SDK/Le Harness ownership split, and short decision records for handwritten API, bounded streaming, response evolution, and retry policy. Compare three representative workflows with an existing SDK for design context without reopening the standalone-project decision.

Fetch the official schema; record source URL, date, digest, and known inconsistencies. Mark wire fixtures as synthetic, SDK-derived, or live-observed. Check intended repository/package naming before making installation claims.

**Accept when:** every v0.1 promise has an owner and a test; unresolved wire questions have explicit investigation tasks; no superiority or full-coverage claim is implied. Depends on no code.

### P01 — Crate, API sketch, and contributor foundation

Create one Cargo library, README, licenses, CONTRIBUTING, changelog, and CI. Select the MSRV and dependency set; expose a small proposed API through compiling examples or temporary compile fixtures, then replace temporary implementations as endpoints land. Do not publish placeholder behavior.

Declare an additive feature policy and target matrix. Enforce formatting, Clippy, library/doc compilation, and no unsafe code in project-owned Rust unless a separately reviewed need emerges. Record MSRV verification and dependency rationale.

**Accept when:** the package builds on the declared compiler, public examples establish consistent ergonomics, and CI has no cloud-key requirement. Depends on P00.

### P02 — Client, authentication, transport, and errors

Implement explicit credentials and an opt-in environment constructor; shared connection pool; versioned base URL handling; content negotiation; configurable deadlines and byte limits. Disable redirects and underlying automatic retries. Implement bounded JSON/non-JSON API error handling, metadata, and redacted Display/Debug.

Test base-path joining, encoded path segments, authorization, invalid configuration, oversized bodies, unusual statuses, and two-host redirect handling. No redirect may forward credentials.

**Accept when:** the mock server observes exact methods/paths/headers; failed generation calls produce exactly one attempt; logs and formatted errors contain no fixture secrets or content. Depends on P01.

### P03 — Models discovery

Implement list/retrieve and declared query parameters. Use string-backed IDs and model variants with preserved additional data. Model capabilities are observations supplied by the provider, not a hidden routing policy or permanent cache.

Cover list envelope, base/fine-tuned cards, optional metadata, new fields, unknown variants, unavailable models, and model IDs requiring path encoding. Resolve the schema/example list-shape discrepancy using official SDK evidence and a live probe when available.

**Accept when:** both public methods work without hardcoded model names, implicit inference, or extra requests. Depends on P02.

### P04 — Complete chat and message fidelity

Implement builders, role-specific messages, complete responses, finish reasons, usage, and metadata. Preserve assistant tool calls and structured content for follow-up requests. Omit unset optional fields; require an explicit model. Document which native fields are typed or available through the checked extension mechanism.

Include simple chat and history-round-trip examples. Test null assistant content, text/block content, multiple choices, unknown finish reasons, malformed required fields, and absent usage values.

**Accept when:** a response can be continued without losing tool IDs or opaque content metadata; malformed provider data is not replaced by success-shaped defaults. Depends on P02; use P03 in live examples.

### P05 — Incremental SSE and chat streaming

Implement byte framing, event decoding, typed deltas, explicit terminal state, deadlines, backpressure, and cancellation by ownership. Evaluate reuse of an SSE parsing dependency against the same tests. A parser passing ordinary examples is insufficient.

Test every two-part byte split of representative streams, one-byte delivery, Unicode, line endings, comments, multiline data, oversized pending frames, malformed JSON, EOF, and DONE without socket closure. Follow the exact terminal-state contract in the companion document.

**Accept when:** interrupted streams cannot appear complete, memory is bounded, and cancellation leaves no SDK-owned producer running. Depends on P04.

### P06 — Tool calls, accumulation, and structured output

Implement function schemas, choices, strict options where supported, matched tool-result messages, and separate complete/delta representations. Accept documented string/object argument forms. Add a bounded, explicit accumulator that tracks choice, message, and tool indices separately; reject ambiguous merges.

Add JSON/schema response modes and explicit output parsing; add predicted-output parameters. Include an application-owned mock tool example. No automatic tool execution, output repair, prompt rewriting, or schema-to-Rust generation.

**Accept when:** complete calls and fragment-shaped fixtures both behave correctly, interleaved calls never cross-contaminate, unknown blocks remain available, and schema serialization matches native field names. Depends on P05.

### P07 — Complete and streaming FIM

Implement native FIM requests through shared transport/stream machinery. Keep a distinct request type with prompt, optional suffix, and FIM options; do not add chat messages or tools. Preserve the native response envelope and provide a fallible text convenience accessor.

Test absent versus empty suffix, empty prompt where structurally allowed, Unicode, output limits, stop settings, finish reasons, usage, disconnects, and explicit model selection. Add complete and streamed examples.

**Accept when:** both methods use the native FIM route, retain diagnostics, and perform no filesystem changes or synthetic prompt transformations. Depends on P05.

### P08 — Resilience and diagnostics integration

Consolidate previously implemented timeout, no-retry, cancellation, resource-limit, and redaction behavior. Add adversarial mock timing, non-SSE 200 responses, partial streams, malformed/error bodies, concurrent requests, and metadata-only tracing behind an optional feature.

Expose Retry-After information where present without treating it as permission to replay. Validate all default limits and deadline semantics. Add regression fixtures for discrepancies found by early live probes.

**Accept when:** failures are classified accurately, no layer restarts generation, and default diagnostics reveal no request/response content. Depends on P06–P07; resilience tests already accompany P02–P07.

### P09 — Documentation and examples

Consolidate quickstart, configuration, endpoint/field coverage, streaming lifecycle, output parsing, FIM, error handling, cancellation, and support policy. All code examples must compile; examples that require inference must be explicitly invoked and use caller-selected model IDs.

Explain that transport completion differs from model truncation and valid JSON differs from correct business output. Include a short comparison of design choices without claims about untested competitors.

**Accept when:** a developer can use the supported subset from the packaged documentation without reading internals; examples cover all six methods. Depends on implemented behavior through P08.

### P10 — API drift and dependency maintenance

Turn the P00 source pin and inventory into a repeatable, manual-triggered maintenance command/report. Classify upstream changes affecting covered paths, referenced schemas, or enums. Keep upstream files immutable; store local compatibility decisions separately with rationale and fixtures.

A changed upstream schema produces a review artifact; it never automatically edits the public API or publishes a release. Demonstrate the check using a synthetic schema change. An eventual scheduled workflow can open review items after maintainers choose the cadence.

**Accept when:** the report identifies affected supported surfaces, unrelated API additions do not fail contributor CI, and schema refreshes are reproducible. Depends on P00/P01 and the implemented coverage inventory.

### P11 — Independent consumer and Le Harness handoff

Create a separately compiled consumer fixture outside the library's internal modules. Exercise model discovery, chat streaming, tool-call collection, explicit tool-result continuation, structured JSON, and FIM. Use the packaged crate in offline tests; use selected live models only in the manual compatibility workflow.

Define the dependency boundary for Le Harness: it receives native data and metadata, implements its own routing/budgets/tool execution, and depends on a tagged or released SDK. Do not build Le Harness itself as a release prerequisite.

**Accept when:** a consumer can perform the intended workflows using public APIs only, with no workspace-only path assumptions or private imports. Depends on P08–P10.

### P12 — Release candidate, package, and v0.1

Review API surface, support matrix, coverage, MSRV, package contents, changelog, and migration notes. Build an extracted package as an independent dependency; run the documented CI checks and a publish dry run. Record live checks against actual account-available models and all six methods plus supported feature workflows.

If live validation is unavailable, keep the candidate explicitly unverified and record the remaining gate; mock tests do not establish provider compatibility. Resolve package ownership and publishing credentials through the normal maintainer release process.

**Accept when:** the checklist in [Validation and releases](validation-and-releases.md) is complete. Actual registry publication is a separate release action, not part of this documentation task. Depends on P11.

## 7. Risks and decision gates

| Risk | Response and decision point |
| --- | --- |
| Schema differs from real service | P00 records it; earliest applicable live probe resolves it; scoped override plus regression fixture |
| Streaming correlation is ambiguous | Preserve raw typed deltas; reject unsafe accumulation; do not guess fragments or identities |
| Scope grows toward the whole platform | Keep the v0.1 inventory fixed; new endpoint requires a later milestone |
| Le Harness requires unexpected behavior | P11 tests a consumer early; add generic transport capability only when application policy stays outside |
| Dependency/MSRV mismatch | P01 compiles the actual set; test MSRV and current stable throughout |
| API changes after release | Dated compatibility evidence, open response types, manual drift review, documented release policy |
| Public API churn | Review through compiling examples before release; use builders and non-exhaustive response types |

## 8. After v0.1

First adopt the crate in Le Harness and collect integration failures as small reproducible SDK issues. Prioritize subsequent work from evidence: embeddings for retrieval, richer content conveniences, optional read-only retries, or transport customization. Add each as a separately tested extension.

A 1.0 release requires sustained downstream use and a stable public streaming/error/configuration contract. Endpoint count alone is not a 1.0 criterion. See [API and runtime contract](api-and-runtime-contract.md) and [Research and references](references.md) before implementation.
