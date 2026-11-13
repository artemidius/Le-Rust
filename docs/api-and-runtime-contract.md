# API and runtime contract

Status: **proposed v0.1 contract, 13 November 2026**. Names below specify the intended interface; they are not an existing executable API. P01 must compile representative consumers before names are frozen.

## 1. Public surface

| Method | Result | Responsibility |
| --- | --- | --- |
| `Client::builder().build()` | `Result<Client, Error>` | Validate explicit configuration; do not contact the service |
| `Client::from_env()` | `Result<Client, Error>` | Explicit convenience reading `MISTRAL_API_KEY`; no dotenv or hidden file lookup |
| `client.models().list(options)` | Models response with metadata | One list operation with declared optional filters |
| `client.models().retrieve(model_id)` | Model card with metadata | Encode the ID as one path segment |
| `client.chat().complete(request)` | Complete chat response with metadata | One generation request |
| `client.chat().stream(request)` | `CompletionStream` | Send request, validate HTTP status/media type, return polled stream |
| `client.fim().complete(request)` | Native FIM completion response with metadata | One native FIM request |
| `client.fim().stream(request)` | `CompletionStream` | Native FIM streamed response |

The client is cheaply cloneable and shares its connection pool. It owns no runtime, agent session, history, file, or detached producer task. Core types should be Send where needed for normal Tokio applications; verify these bounds with compile tests. An owned stream must remain usable after a temporary resource accessor is dropped.

Resource methods accept a validated request plus optional per-request options through a consistent builder or method pattern selected in P01. Per-request options may narrow deadlines and limits. Transport flags such as `stream` are set by the chosen method and are not contradictory user-settable booleans.

A supported API call does not automatically discover models, mutate prompts, choose sampling parameters, refresh credentials, execute functions, validate business actions, or retry model output. The API mechanisms are documented in the [source index](references.md); behavior policies in this document are Le Rust design decisions.

## 2. Request types and builders

Require an explicit model string for chat and FIM. Do not provide a model enum or default alias. Use constructors/builders with private fields so adding optional settings does not break consumer struct literals. Avoid infallible constructors that panic on malformed caller configuration.

Optional settings are omitted when unset. An explicit-null facility is introduced only for fields where the pinned contract distinguishes null from omission; do not serialize every missing option as null. Request serialization must be covered by exact JSON fixtures.

Local validation checks structure and stable wire constraints, not a changing model catalog. Examples: required identifiers, valid JSON schemas as JSON values, finite numeric inputs, and a token-count integer representable by the native schema. Do not invent global output limits, reject all empty FIM prefixes, or require every assistant message to contain text. Capability checks are caller-initiated observations and may become stale.

Chat will provide typed options for the supported sampling/token controls, stop conditions, tool definitions/choice, parallel-tool flag, response format, prediction, and declared metadata/cache fields. Maintain a field inventory for the remaining native options. An explicit extension map may pass additional JSON fields; it must reject collisions with every reserved native field, including fields currently omitted. It must never override credentials, endpoint, or stream mode. Extension transmission is not a promise that the service accepts it.

FIM has its own request type: model, prompt, optional suffix, and documented completion controls. It has no chat history, tool definitions, or separate edit instruction. The SDK forwards the supplied content without generating comments, selecting code spans, stripping code fences, or applying edits.

## 3. Messages, tools, and structured outputs

Use role-specific message types. Preserve distinctions among absent, null, textual, and block content where the native schema permits them. A response-to-history conversion must retain tool IDs, structured blocks, and opaque replay metadata, including signatures if supplied. A text accessor must not silently discard non-text content and present the result as a complete replayable message.

Function tools contain their name, optional description, JSON parameter schema, and supported strictness setting. Tool choice covers the native choices and named-function form. A convenient tool-result constructor requires the original tool-call ID. The SDK does not generate replacement IDs or truncate them to a presumed provider-wide length.

Represent complete tool arguments in their documented string or object form without double encoding. Offer an explicit `parse_arguments<T>()` helper. Failure to parse model-generated arguments is an output-data error, not a reason to repeat the API call. Applications own JSON Schema validation, authorization, and function execution.

Response-format options represent text, JSON object, and JSON Schema modes. Serialize nested schema data using the provider's wire key `schema`, independently of Rust/generator field names. Accept caller-supplied schema JSON; schema generation from Rust types can be a later optional convenience. A typed deserializer verifies deserializability, not arbitrary schema compliance or business correctness. JSON-mode examples must include the caller's explicit JSON instruction.

Predicted content belongs to the chat request. Pass it explicitly and expose returned usage without inferring a speed or cost saving. Record feature/model compatibility from live checks rather than a hardcoded allowlist.

## 4. Response evolution

Provider responses must tolerate additional fields. Use open string enums such as `FinishReason::Unknown(String)` and preserve unknown content blocks as JSON. Extensible public enums should be non-exhaustive; use constructors/accessors for types whose fields may grow. Serde's `deny_unknown_fields` is inappropriate for provider responses.

Unknown values must stay unknown: missing token usage is not zero; an absent capability flag is not confidently false. Preserve extra usage details without inventing their billing meaning. Known required fields with incorrect types still produce a decode error. A tolerant decoder must not manufacture a successful response from malformed data.

Store native complete responses separately from stream chunks. The completion family may contain more than a single text message or choice. Keep metadata and the original structured contents accessible even where a convenience view supports only text. Unsupported aggregation can fail explicitly while the low-level stream remains usable.

## 5. Transport and configuration

The default versioned base URL is `https://api.mistral.ai/v1/`. Append relative endpoint paths without losing `/v1/` or duplicating it. Validate base URLs; reject user-info credentials, query strings, and fragments. Require HTTPS except for explicitly selected loopback HTTP mock/development endpoints. Custom endpoints are deliberate destinations for the configured credentials, not evidence of full compatibility with another provider.

Authentication is a sensitive bearer header. Do not put keys into query strings or URLs. Disable redirects for the SDK-owned HTTP client and test a redirect to another host. Do not expose a general prebuilt-client injection path in v0.1: invisible retry/redirect behavior would weaken the public guarantees. Revisit explicit transport customization when a concrete consumer requires it.

All requests use declared content negotiation. Validate a successful stream's media type, including ordinary parameters such as charset. A 200 HTML page or unexpected JSON response to a streaming method is an explicit error, not a zero-token successful stream. Parse HTTP failures before treating the body as SSE.

Proposed configurable SDK defaults, to be measured and confirmed in P08:

| Resource/deadline | Initial value | Meaning |
| --- | --- | --- |
| Connect timeout | 10 seconds | Connection establishment |
| Response-header deadline | 60 seconds | Awaiting the initial HTTP response |
| Stream read inactivity | 60 seconds | Waiting for additional bytes while consuming the response |
| Overall operation deadline | 10 minutes | Request start through complete body consumption; includes caller pauses |
| Complete JSON body | 16 MiB | Decoded-body ceiling, independent of Content-Length |
| HTTP error body | 64 KiB | Stored diagnostic maximum; preserve a truncation flag |
| Pending SSE line/event | 1 MiB each | Reject before an unfinished frame can grow without bound |
| Accumulated completion | 16 MiB | Applies only to explicit collection |
| Assembled tool arguments | 1 MiB per call | Bound unclosed/unfinished argument data |

These are client policy limits, not Mistral limits. The caller may configure them. Define overlapping timeout precedence so the earliest expired applicable deadline wins. Report a stage only when known; do not label every connection failure a timeout. Expired deadlines become visible on the next poll in a task-free stream. The SDK cannot force an application to poll or release a retained stream.

Count decoded bytes and framing state; do not rely on a Content-Length header or assume network chunks are small. Account for multiple queued events when bounding the parser. Read only as the consumer polls; do not accumulate an entire stream in the background. Underlying HTTP/socket buffers still exist and are outside a claim of exact process memory size.

## 6. Retry contract

**v0.1 performs no automatic retries on any method.** Enforce this at the SDK layer and configure the selected reqwest version to disable its internal retry policy. Reqwest currently has default protocol retry behavior; see [references](references.md).

Preserve HTTP status and parsed Retry-After information when present, including malformed/unrecognized values as diagnostic data. A transient condition and safe repeatability are different facts. Do not expose a blanket `is_retryable()` that promises a generation can safely be repeated.

A lost response does not establish that the server never generated or billed output. No idempotency guarantee was verified in this research. Future retry support must be explicit and bounded; read-only model queries are the first candidate. Never reconnect a partial generation, resend it transparently, or combine output from two attempts. SSE `retry:` and `id:` fields do not enable browser-style reconnection here.

## 7. Streaming protocol

Keep the layers independent:

```text
HTTP bytes -> incremental SSE framing -> event data -> native JSON -> typed delta
```

Use the SSE framing rules for blank lines, comments, optional spaces, line endings, multiline data, and an initial BOM. Test UTF-8 characters and CRLF split across chunks. Do not use lossy decoding independently on each network chunk. Le Rust will reject invalid UTF-8 in protocol data explicitly; that is a stricter SDK validation choice rather than a claim of complete browser-decoder equivalence.

The SSE `data:` payload is the native completion-chunk JSON. SDK wrapper objects containing a `data` property must not be mistaken for an extra wire envelope. Keep separate types for complete tool calls and optional stream deltas.

| Observation | Required behavior |
| --- | --- |
| Complete JSON data event | Yield one decoded chunk; retain unknown supported extension data |
| Comment/irrelevant SSE field | Handle framing; do not expose as generated text |
| `data: [DONE]` event | Mark protocol completion, release the body, and finish the stream |
| Choice finish reason | Preserve it and keep reading; other choices or usage may follow |
| Finish reason such as length | Valid received model result indicating truncation; not a transport failure |
| EOF before DONE | One terminal UnexpectedEof error, even if a choice previously finished |
| Malformed JSON, invalid UTF-8, ambiguous event, or limit exceeded | One terminal error; stop polling the body |
| HTTP disconnect after output | Preserve already yielded data as partial; return terminal failure |
| DONE followed by a connection that stays open | Finish immediately without waiting for socket closure |

After a terminal result, future polls return no more items. An incomplete trailing SSE event is not dispatched at EOF. An unfamiliar event kind should remain inspectable or produce an explicit unsupported-protocol error; it must not disappear as apparent success.

## 8. Explicit response collection

Low-level streaming is the primary contract. A separate bounded accumulator can build a completion for consumers that want it. It must:

- Track choice index, message index, and tool-call index as distinct identities.
- Keep calls to the same function separate; never group by name or arrival position alone.
- Append actual string fragments exactly; preserve complete object arguments without concatenating or double-encoding them.
- Treat IDs and names according to the observed native shape; reject conflicting identity rather than guessing.
- Preserve structured blocks, finish reasons, metadata, and the last applicable usage observation without summing cumulative reports.
- Reject unsupported content assembly explicitly, while keeping original deltas available through the low-level interface.
- Return a successful collected completion only after DONE and structural validation of the collected response.

Fragment-shaped tool payloads are defensive test cases; research did not establish that every hosted Mistral tool call is fragmented. Resolve complete-versus-fragment behavior with sanitized live fixtures. A collector error may carry explicitly incomplete content behind an opt-in accessor, never a normal success response. Terminal transport success does not make tool arguments valid JSON, and valid JSON does not make the requested action authorized.

## 9. Cancellation

Dropping the owned request future or owned stream releases the SDK's local operation. There is no detached producer to keep generating requests or buffering content. Dropping only a temporary `next()` future leaves the stream itself alive; documentation and examples must explain the difference.

The SDK does not promise provider-side cancellation, stopped inference, or avoided billing. Test local resource cleanup and absence of SDK-owned work, not an unverifiable claim about remote execution. If a future implementation introduces tasks, explicit cancellation and joining become part of ownership; dropping a JoinHandle alone is not cancellation.

## 10. Errors and diagnostics

Use an extensible error taxonomy: Configuration, InvalidRequest, Transport, Timeout, HttpApi, Decode, StreamProtocol, UnexpectedEof, LimitExceeded, and Accumulation. Preserve an inspectable source where useful without forcing all public types to expose reqwest-specific details.

HTTP errors retain status, optional provider error type/code, optional request ID, retry metadata, and a bounded original body. Support documented JSON error families, validation-detail arrays, and non-JSON proxy/service failures. Do not require a particular request-ID header on every response.

Default Display, Debug, tracing, and error source formatting must omit credentials and request/response content. Provider messages and validation bodies may echo submitted data; access them only through clearly named explicit diagnostic accessors. Redaction means avoiding default disclosure, not claiming arbitrary text can always be sanitized perfectly.

Metadata-only diagnostics may include endpoint identifier, status, duration, byte counts, attempt count, and a safe request identifier where present. Never install a global tracing subscriber, write logs/files, or enable telemetry automatically. Caller-supplied content remains caller-owned.

## 11. Le Harness boundary

| SDK returns/provides | Harness decides/executes |
| --- | --- |
| Available models and capabilities | Which model to use |
| Native requests, responses and usage | Context construction, budgets and cost calculations |
| Tool schemas, calls and result messages | MCP discovery, authorization and actual tool calls |
| FIM candidate content | Span selection, prompt strategy, file validation and application |
| Stream state and errors | UI behavior, explicit retries and partial-result recovery |

Both SDK examples and the independent consumer fixture must preserve this boundary. The SDK needs no Git access, sandbox, filesystem editing, model routing, conversation persistence, or automatic agent loop.
