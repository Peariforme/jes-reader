# Releasing

For maintainers.

The `Package` workflow builds the VSIX and CLI archives for five platforms on every pull request and push to `main` (run artifacts). To release:

1. Set the same version in `editors/vscode/package.json`, in `Cargo.toml` (`workspace.package.version`) and in the `jes-reader-core` dependency of the root `Cargo.toml` (crates.io needs it). Add the release to `editors/vscode/CHANGELOG.md`.
2. Merge, then push a `vX.Y.Z` tag on `main`. The workflow rejects a tag that does not match the versions.

The workflow then does the rest:

- the GitHub release, with every VSIX and CLI archive, a `SHA256SUMS` file and a build-provenance attestation for each asset;
- the VS Code Marketplace and Open VSX, one VSIX per platform;
- crates.io: `jes-reader-core`, `jes-reader-lsp`, then `jes-reader-cli`.

No account or token is needed. Every registry trusts this repository's `package.yml` workflow through [Trusted Publishing](https://docs.github.com/en/actions/security-for-github-actions/security-hardening-your-deployments/about-security-hardening-with-openid-connect): GitHub proves the workflow with an OIDC token, exchanged for a short-lived publishing token. Each registry has its own switch, a repository variable: `PUBLISH_MARKETPLACE`, `PUBLISH_OPENVSX` and `PUBLISH_CRATES`, all `true`.

The jobs can be re-run: an existing release gets its assets replaced, and a version already published to a registry is skipped.

`cargo publish --workspace --dry-run` runs in CI so that the crates stay publishable.

## Signing

The Marketplace and Open VSX sign each VSIX they receive, and VS Code checks the Marketplace signature on install.

The VSIX of the GitHub release, for offline workstations, are checked with the attestation or the checksums:

```sh
gh attestation verify jes-reader-win32-x64-X.Y.Z.vsix --repo Peariforme/jes-reader
sha256sum -c SHA256SUMS --ignore-missing
```

## Documentation site

The `Docs` workflow builds this site with mdBook on every pull request, and deploys it to GitHub Pages on every push to `main`.
