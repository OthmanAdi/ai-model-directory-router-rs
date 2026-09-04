# Security Policy

## Supported Versions

Security fixes are provided for the latest published minor release.

| Version | Supported |
|---------|-----------|
| 0.3.x | Yes |
| 0.2.x and earlier | No |

## Reporting a Vulnerability

Please report suspected vulnerabilities privately through
[GitHub Security Advisories](https://github.com/OthmanAdi/ai-model-directory-router-rs/security/advisories/new).
Do not open a public issue for an undisclosed vulnerability.

Include the affected crate version, Rust version, impact, reproduction steps,
and any suggested mitigation you can safely share. The maintainer will use the
advisory to coordinate investigation, fixes, and disclosure with you.

## Scope

Security reports may cover catalog parsing, file and network loading, routing,
cost estimation, or dependency risks in this crate. Provider authentication,
provider API clients, and model inference are outside this crate's scope.
