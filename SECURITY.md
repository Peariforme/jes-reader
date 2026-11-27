# Security policy

## Supported versions

Security fixes go into the latest release. Please update before reporting.

## Reporting a vulnerability

Please do not open a public issue for a vulnerability. Report it privately through [GitHub's private vulnerability reporting](https://github.com/Peariforme/jes-reader/security/advisories/new).

Include what an attacker can do, the affected component (VS Code extension, language server or CLI) and version, and a minimal example to reproduce it. **Do not include a real sysout from your site**: rebuild a minimal example with the names changed.

The report stays private until a fix is released.

## Scope

JES Reader reads files and documents on the workstation; it sends nothing over the network and connects to nothing on z/OS (see [Privacy and security](https://peariforme.github.io/jes-reader/reference/privacy.html)). Of particular interest:

- a sysout or `.jesreader.toml` that makes the language server or the CLI crash, hang, or read or write files it should not;
- a link built from a sysout that opens something other than the data set it names.
