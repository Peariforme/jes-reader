# Releasing

For maintainers.

The `Package` workflow builds the VSIX and CLI archives for five platforms on every pull request and push to `main` (run artifacts). To release:

1. Set the same version in `editors/vscode/package.json`, in `Cargo.toml` (`workspace.package.version`) and in the `jes-reader-core` dependency of the root `Cargo.toml` (crates.io needs it). Add the release to `editors/vscode/CHANGELOG.md`.
2. Merge, then push a `vX.Y.Z` tag on `main`. The workflow rejects a tag that does not match the versions.
3. The workflow creates the GitHub release with every VSIX and CLI archive attached, then publishes:
   - to the VS Code Marketplace, when the repository variable `PUBLISH_MARKETPLACE` is `true`;
   - to Open VSX, when the `OVSX_PAT` secret exists;
   - the three crates to crates.io, when the variable `PUBLISH_CRATES` is `true`.

The release also carries a `SHA256SUMS` file and a GitHub build-provenance attestation for every asset (see [Signing](#signing)).

The jobs can be re-run: an existing release gets its assets replaced, and a version already on the Marketplace, Open VSX or crates.io is skipped.

## VS Code Marketplace

Publishing uses [Trusted Publishing](https://code.visualstudio.com/api/working-with-extensions/publishing-extension): GitHub proves the repository and workflow with an OIDC token (`vsce publish --oidc`, vsce 4), so no token is stored. Setup, once:

1. Create the `peariforme` publisher on [marketplace.visualstudio.com/manage](https://marketplace.visualstudio.com/manage). The ID must match `publisher` in `editors/vscode/package.json`.
2. If Trusted Publishing cannot create a new extension, upload the first VSIX of each platform by hand from the same page.
3. On the publisher, under *Trusted Publishing*, add a policy for the GitHub repository `Peariforme/jes-reader` and the workflow `package.yml`.
4. In the repository settings, add the Actions variable `PUBLISH_MARKETPLACE` = `true`.

## crates.io

Publishing uses [Trusted Publishing](https://crates.io/docs/trusted-publishing) as well: `rust-lang/crates-io-auth-action` exchanges GitHub's OIDC token for a short-lived publish token. Crates are published in dependency order: `jes-reader-core`, `jes-reader-lsp`, `jes-reader-cli`. Setup, once:

1. A crate must exist before Trusted Publishing can be set up on it: publish the first version by hand, from the tagged commit, with an API token from [crates.io/settings/tokens](https://crates.io/settings/tokens): `cargo login`, then `cargo publish --workspace`.
2. On each of the three crates, under *Settings → Trusted Publishing*, add the GitHub repository `Peariforme/jes-reader` and the workflow `package.yml`. Then revoke the API token.
3. In the repository settings, add the Actions variable `PUBLISH_CRATES` = `true`.

`cargo publish --workspace --dry-run` runs in CI so that the crates stay publishable.

## Open VSX

[Open VSX](https://open-vsx.org) is the registry used by VSCodium, Eclipse Theia, Gitpod and other VS Code forks. Open VSX has no Trusted Publishing, so publishing uses a token stored as the `OVSX_PAT` secret. Setup, once:

1. Create an Eclipse account on [accounts.eclipse.org](https://accounts.eclipse.org/user/register), and fill in your GitHub username in its profile.
2. Log in to [open-vsx.org](https://open-vsx.org) with GitHub. In *Settings → Profile*, log in with the Eclipse account, then sign the Eclipse Foundation Open VSX Publisher Agreement.
3. In *Settings → Access Tokens*, create a token.
4. Create the namespace, which must match `publisher` in `editors/vscode/package.json`: `npx ovsx create-namespace peariforme -p <token>`.
5. In the repository settings, add the Actions secret `OVSX_PAT` with the token.
6. Claim the namespace so that its extensions show as verified: open a *Claim namespace ownership* issue on [EclipseFdn/open-vsx.org](https://github.com/EclipseFdn/open-vsx.org/issues/new/choose).

To publish a version released before this setup, download its VSIX from the GitHub release and run `npx ovsx publish --packagePath *.vsix -p <token>`.

## Signing

There is no certificate to buy or key to manage:

- **VS Code Marketplace.** The Marketplace signs every VSIX it receives. VS Code checks that signature on install and refuses a package that has been altered.
- **Open VSX.** Open VSX signs the VSIX on publication as well.
- **VSIX from the GitHub release** (offline workstations). No store signs them. Each asset is listed in `SHA256SUMS` and has a provenance attestation signed by GitHub for this repository and workflow. To check a VSIX: `gh attestation verify jes-reader-win32-x64-X.Y.Z.vsix --repo Peariforme/jes-reader`, or compare its `sha256sum` with `SHA256SUMS`.

The blue *verified publisher* check mark on the Marketplace is a different thing: it proves that the publisher owns a domain. On [marketplace.visualstudio.com/manage](https://marketplace.visualstudio.com/manage), under the publisher's *Details*, enter a domain you own (not `github.io`) and add the DNS TXT record shown. Microsoft then reviews the request; see the [eligibility rules](https://code.visualstudio.com/api/working-with-extensions/publishing-extension#verify-a-publisher).

## Documentation site

The `Docs` workflow builds this site with mdBook on every pull request, and deploys it to GitHub Pages on every push to `main`.
