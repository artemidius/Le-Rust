# Le Rust

An independent, asynchronous Rust SDK for selected native Mistral APIs.

**Status: implementation planning, 13 November 2026. No SDK code, package, live API validation, or release exists in this folder.**

Le Rust will provide typed requests, complete and streaming responses, model discovery, tool-call payloads, and Codestral FIM. Le Harness will be its first application consumer. The SDK will also work independently in other Rust applications.

The selected product name is **Le Rust**. The intended Cargo package is `le-rust`, imported as `le_rust`; package and repository availability must be checked before publication. Le Rust is unofficial and is not affiliated with Mistral AI.

## Implementation documents

- [Detailed implementation plan](docs/implementation-plan.md): decisions, scope, milestones, PR backlog, dependencies, estimates, and completion gates.
- [API and runtime contract](docs/api-and-runtime-contract.md): public API, wire types, streaming, cancellation, errors, resource limits, and SDK/application boundaries.
- [Validation and releases](docs/validation-and-releases.md): test matrix, CI, live compatibility checks, packaging, maintenance, and release criteria.
- [Research and references](docs/references.md): dated primary sources, schema discrepancies, evidence limits, and decisions that require implementation-time verification.

## First release

| Resource | Planned methods |
| --- | --- |
| Models | List and retrieve |
| Chat | Complete and stream |
| FIM | Complete and stream |

Chat will include function-tool payloads, JSON/schema output parameters, and predicted-output parameters. Coverage will be declared at field and content-type level; support for these operations will not imply support for every Mistral product or every model capability.

The initial implementation will use one Rust library crate, Tokio, reqwest, and Serde. Public API design will be handwritten and checked against a pinned official OpenAPI source. No TypeScript component is planned.

Le Harness will own model routing, tool execution, MCP, file editing, sandboxing, persistence, and evaluation. Le Rust will transport API data and expose the evidence applications need to make those decisions.

## Delivery principle

Ship a tested, documented v0.1 subset, then use it in Le Harness. Full platform coverage and a 1.0 release are later decisions. Existing community SDKs are acknowledged in the references; this project makes no first-of-its-kind or unmeasured superiority claim.
