# API and runtime contract

This document describes the behavior implemented by Le Rust 0.1. The public Rust API remains pre-1.0, but the operational guarantees below are intentional compatibility boundaries.

## Public surface

| Operation | Rust entry point |
| --- | --- |
| List models | `client.models().list(ListModelsOptions)` |
| Retrieve a model | `client.models().retrieve(model_id, RequestOptions)` |
| Complete chat | `client.chat().complete(ChatRequest, RequestOptions)` |
| Stream chat | `client.chat().stream(ChatRequest, RequestOptions)` |
| Complete FIM | `client.fim().complete(FimRequest, RequestOptions)` |
| Stream FIM | `client.fim().stream(FimRequest, RequestOptions)` |

Every generation request requires a caller-supplied model identifier. Resource methods do not perform hidden discovery calls. `Client::from_env()` reads only `MISTRAL_API_KEY`; it does not load dotenv files or other credential stores.

Request constructors use private fields and chainable setters. Optional values are omitted when unset. Chat's extension map can carry a native field that is not yet typed, but it rejects keys reserved by the typed request and can never override the endpoint, authentication, or transport-controlled `stream` value.

## Messages, tools, and output formats

`Message` distinguishes system, user, assistant, and tool roles. Complete assistant responses can be converted back to request messages without flattening tool calls or structured content. `Content::Blocks` preserves content variants the SDK does not interpret.

Function tools carry the native JSON parameter schema. Tool arguments preserve both documented string and JSON representations; `parse_arguments<T>()` is explicit and fallible. The SDK never executes a returned tool call. Callers are responsible for schema validation, authorization, and side effects.

`ResponseFormat` supports text, JSON object, and native JSON Schema modes. Successful deserialization proves only that output matches the requested Rust shape; applications still own business validation. Predicted content is forwarded explicitly and is never selected automatically.

## Transport guarantees

- The default base URL is `https://api.mistral.ai/v1/`.
- Custom base URLs reject user information, query strings, and fragments.
- Plain HTTP is opt-in and restricted to loopback addresses.
- Bearer credentials are stored in a sensitive header value and omitted from `Debug`.
- HTTP redirects and reqwest's underlying retries are disabled.
- One public operation performs one HTTP request.
- HTTP error status, request ID, Retry-After, and a bounded diagnostic body are retained. The body is available only through `ApiError::diagnostic_body()` because providers may echo submitted content.

The default limits are client policy, not provider limits:

| Resource | Default |
| --- | ---: |
| Complete response body | 16 MiB |
| Stored HTTP error body | 64 KiB |
| Pending SSE line | 1 MiB |
| SSE event | 1 MiB |
| Decoded events queued from one network chunk | 256 |
| Explicitly collected completion content | 16 MiB |
| Assembled arguments for one tool call | 1 MiB |

`Limits` can be changed before client construction. `RequestOptions` can narrow the response-header deadline, operation deadline, or complete-response limit for one call.

## Streaming

The decoder processes `HTTP bytes -> SSE frames -> JSON data -> CompletionChunk`. It supports LF, CRLF, an initial UTF-8 BOM, comments, and multiline `data` fields without assuming network-chunk boundaries match lines or Unicode characters.

`data: [DONE]` is the only successful transport terminator. A finish reason such as `stop` or `length` is preserved as model output and does not end the stream. EOF before `[DONE]` yields `UnexpectedEof`; malformed data, unexpected media type, transport errors, and limit violations each yield one terminal error followed by stream exhaustion.

`CompletionStream::collect_response()` is an optional bounded accumulator. It keeps choices and tool calls separate by index, rejects conflicting identities or content representations, and succeeds only after protocol completion. Low-level consumers can instead process native chunks directly.

Dropping the owned stream stops local consumption and releases SDK-owned state. It cannot prove that remote inference stopped or that billing was avoided.

## Error and evolution policy

`ErrorKind` provides stable categories for configuration, request validation, transport, timeout, HTTP API, decode, stream protocol, premature EOF, resource limits, and accumulation. Provider response fields are forward-tolerant: unknown finish reasons and extra fields remain inspectable, missing usage remains missing, and malformed required fields still fail decoding.

Default `Display`, `Debug`, and optional tracing omit credentials, prompts, generated content, tool arguments, and provider diagnostic bodies. Tracing contains endpoint metadata and status only.
