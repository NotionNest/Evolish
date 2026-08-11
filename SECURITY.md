# Security Policy

## Supported versions

Before the first public binary release, security fixes are applied to `main`. After releases begin, the latest published release and `main` receive security fixes; older releases are unsupported unless a release advisory explicitly says otherwise.

| Version | Supported |
|---|---|
| `main` | Yes |
| Latest public release | Yes, once available |
| Older releases | No |

## Reporting a vulnerability

Do not open a public Issue for a suspected vulnerability or include credentials, private query content, or exploit details in public discussions. Use [GitHub private vulnerability reporting](https://github.com/NotionNest/Evolish/security/advisories/new).

Include the affected version or commit, platform, prerequisites, reproducible steps, impact, and any suggested mitigation. Use synthetic credentials and redact personal content.

The maintainer aims to acknowledge a report within three business days and provide a triage update within seven business days. Resolution and disclosure timing depend on severity, exploitability, platform coordination, and upstream fixes. Reporters will be credited when desired and when coordinated disclosure permits it.

## Scope and dependency policy

High-value areas include Tauri IPC/capabilities, credential storage, update verification, deep links, file and URL handling, provider authentication, logging redaction, cross-application capture, and local data access.

CI checks npm advisories, RustSec advisories, Rust and JavaScript dependency licenses, and dependency sources. Any advisory exception must name an exact advisory ID, explain reachability and the upstream constraint, and be removed as soon as a safe upgrade exists.
