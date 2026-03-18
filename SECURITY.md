# Security Policy

**English** | [한국어](docs/SECURITY.ko.md)

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| latest  | :white_check_mark: |

As CADKernel is in early development, security updates are applied to the latest version on the `main` branch.

## Reporting a Vulnerability

**Please do NOT report security vulnerabilities through public GitHub issues.**

Instead, please report them through [GitHub Security Advisories](https://github.com/kernalix7/CADKernel/security/advisories/new).

### What to Include

When reporting a vulnerability, please include:

1. **Description** — A clear description of the vulnerability
2. **Steps to Reproduce** — Detailed steps to reproduce the issue
3. **Impact** — The potential impact of the vulnerability (e.g., data loss, arbitrary code execution)
4. **Affected Components** — Which parts of CADKernel are affected (e.g., file parser, plugin system, MCP server)
5. **Environment** — OS, Rust version, CADKernel version

### Response Timeline

- **Acknowledgment** — Within 48 hours of the report
- **Initial Assessment** — Within 7 days
- **Fix & Disclosure** — Coordinated with the reporter; typically within 30 days for critical issues

### Scope

The following areas are considered in-scope for security reports:

- Memory safety issues (despite Rust's guarantees, `unsafe` blocks may exist)
- File format parser vulnerabilities (malicious file handling)
- Plugin/Add-on sandboxing escapes
- MCP server authentication/authorization issues
- Denial of service through crafted inputs
- Supply chain concerns (dependency vulnerabilities)

### Out of Scope

- Bugs that require physical access to the user's machine
- Social engineering attacks
- Issues in third-party dependencies (please report these upstream, but let us know)

## Security Best Practices

CADKernel follows these security practices:

- **Minimal `unsafe` usage** — All `unsafe` blocks are documented and reviewed
- **Dependency auditing** — Regular `cargo audit` checks for known vulnerabilities
- **Fuzz testing** — Continuous fuzz testing of file parsers and input handling
- **Sandboxed plugins** — Add-ons run in isolated environments

## Acknowledgments

We appreciate the security research community's efforts in responsibly disclosing vulnerabilities. Contributors who report valid security issues will be acknowledged (with permission) in our release notes.

---

*This security policy is subject to change as the project matures.*
