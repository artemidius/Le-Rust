# Security policy

## Reporting a vulnerability

Please report suspected vulnerabilities through GitHub's private security advisory flow for this repository. Do not open a public issue containing credentials, private prompts, authorization headers, or an exploit that could affect active users.

Include the affected version, impact, reproduction steps, and any suggested mitigation. Reports will be acknowledged as soon as practical, then investigated and coordinated before public disclosure.

## Security boundaries

Le Rust transports caller-provided content to configured API endpoints. It does not execute model-requested tools, edit files, or authorize application actions. Callers must validate and authorize model-generated arguments before executing them.

Provider error bodies may echo submitted content. Le Rust excludes them from default `Debug` and `Display` output, but exposes them through an explicit diagnostic accessor. Applications should treat that accessor as sensitive.

Dropping a local stream releases SDK-side resources; it does not prove that provider-side generation stopped or that billing was avoided.
