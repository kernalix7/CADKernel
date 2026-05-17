# Security Policy

## Supported Versions

CADKernel is pre-1.0 software. Security fixes are applied to the active
development branch and the latest tagged release line when a release exists.

| Version | Supported |
| --- | --- |
| `main` | Yes |
| Latest tagged release | Yes |
| Older snapshots and abandoned branches | No |

## Reporting a Vulnerability

Report suspected vulnerabilities privately by e-mailing
`kernalix7@kodenet.io`. Include:

- A concise description of the issue and affected component.
- Reproduction steps, proof-of-concept input, or crash details when available.
- The expected impact, such as code execution, sandbox escape, denial of
  service, file disclosure, corrupted CAD output, or supply-chain compromise.
- Any disclosure deadline or coordination constraints.

If encrypted communication is needed, say so in the initial message and provide
your preferred public key or request a maintainer key before sending sensitive
details.

Do not file public GitHub issues for undisclosed vulnerabilities.

## Response Timeline

CADKernel uses the following target response windows:

| Milestone | Target |
| --- | --- |
| Initial acknowledgement | Within 48 hours |
| Triage and severity assessment | Within 7 days |
| Patch or mitigation available | Within 30 days when practical |
| Coordinated public disclosure | Within 90 days unless active exploitation or reporter constraints require a different schedule |

If a fix requires a dependency update, design change, or external maintainer
coordination, the maintainer will provide status updates until resolution.

## Advisory Process

When a report is confirmed, the maintainer will:

1. Assign severity based on exploitability, affected platforms, and data or
   model integrity impact.
2. Prepare a private fix and regression test.
3. Coordinate the disclosure date with the reporter.
4. Publish a GitHub Security Advisory when appropriate.
5. Request or attach CVE identifiers for issues that meet CVE criteria.
6. File a RustSec advisory when a published Rust crate is affected and the
   issue is relevant to downstream consumers.

Supply-chain issues are tracked through `cargo deny`, `cargo vet`, and security
advisory monitoring. Vulnerable dependencies are patched, replaced, or
temporarily documented with an explicit risk assessment.

## Past Advisories

| Advisory | Date | Affected versions | Fixed version | Summary |
| --- | --- | --- | --- | --- |
| None | - | - | - | No published CADKernel advisories yet. |
