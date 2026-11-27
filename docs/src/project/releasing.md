# Releasing

For maintainers.

The `Package` workflow builds the VSIX and CLI archives for five platforms on every pull request and push to `main` (run artifacts). To release:

1. Set the same version in `editors/vscode/package.json`, in `Cargo.toml` (`workspace.package.version`) and in the `jes-reader-core` dependency of the root `Cargo.toml` (crates.io needs it). Add the release to `editors/vscode/CHANGELOG.md`.
2. Merge, then push a `vX.Y.Z` tag on `main`. The workflow rejects a tag that does not match the versions.
3. The workflow creates the GitHub release with every VSIX and CLI archive attached, then publishes:
   - to the VS Code Marketplace, when the repository variable `PUBLISH_MARKETPLACE` is `true`;
   - to Open VSX, when the `OVSX_PAT` secret exists;
   - the three crates to crates.io, when the variable `PUBLISH_CRATES` is `true`.

The jobs can be re-run: an existing release gets its assets replaced, and a crate version already on crates.io is skipped.

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

`OVSX_PAT` is an open-vsx.org token, after creating the namespace once with `npx ovsx create-namespace peariforme`.

## Documentation site

The `Docs` workflow builds this site with mdBook on every pull request, and deploys it to GitHub Pages on every push to `main`.
