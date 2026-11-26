# jes-reader

A language server that turns a z/OS JES2 sysout into a navigable document: failures in the Problems panel, job steps in the Outline, messages colored by severity. Everything runs on your workstation; nothing is installed or opened on z/OS.

It works on the spool files you open from [Zowe Explorer](https://marketplace.visualstudio.com/items?itemName=Zowe.vscode-extension-for-zowe), one DD at a time, on files you have downloaded, and on whole spools saved in one file (BMC AMI DevX JES Explorer).

> Status: phases 1 to 3 are done — parser, language server, CLI, VS Code extension, Zowe Explorer integration, hovers, house rules and job summaries. See [docs/ROADMAP.md](docs/ROADMAP.md).

## What it does

- **Diagnostics**: abends (`S0C7`, `U4038`…) and JCL errors as errors, return codes above a threshold (4 by default) as warnings, steps that did not run as information. Severity is not read from the message suffix alone: `IEF453I` (JCL ERROR) and `IEF472I` (abend) both end in `I`.
- **Outline**: steps with their outcome and program in `JESMSGLG` and `JESYSMSG`; a `JOB → EXEC → procedure EXEC → DD` tree in `JESJCL`.
- **Step summaries**: a code lens above each step of `JESYSMSG` with its program, outcome, CPU and elapsed time.
- **House rules** (`.jesreader.toml`): message severities, return codes tolerated by step name or by program.
- **CLI** for scripts and CI: `jes-reader check` (text, JSON or SARIF) and `jes-reader summary` (text or JSON).
- **Folding** of steps, JCL statements and multi-line messages.
- **Syntax highlighting** of message IDs, abend codes, return codes, dataset names and JCL statements.

## Install

- **VS Code**: the [JES Reader extension](https://marketplace.visualstudio.com/items?itemName=peariforme.jes-reader) bundles the language server. For a workstation without internet access, download the VSIX of your platform from a [GitHub release](https://github.com/Peariforme/jes-reader/releases) and run *Extensions: Install from VSIX…*; no admin rights are needed.
- **CLI**: `cargo install jes-reader-cli` (binary `jes-reader`), or an archive from a GitHub release.
- **Language server** for another editor: `cargo install jes-reader-lsp`.

## Repository layout

| Path | Contents |
| --- | --- |
| `crates/jes-reader-core` | Parser, `Job → DD → step → message` model, rule engine |
| `crates/jes-reader-lsp` | Language server (`lsp-server`, synchronous) |
| `crates/jes-reader-cli` | `jes-reader` CLI (`check` and `summary` on files or directories) |
| `editors/vscode` | VS Code extension (thin client, bundled server) |
| `corpus/` | Test JCL and the sysouts they produce — no customer data |
| `docs/` | Roadmap and sysout format notes |

## Development

Once after cloning, enable the Git filter that anonymizes the corpus on `git add` (IBM Z Xplore user IDs become `Z99999`):

```sh
git config filter.anonymize.clean "sh scripts/anonymize.sh"
git config filter.anonymize.smudge cat
git config filter.anonymize.required true
```

Your working copy keeps the real values; only what is committed is anonymized. CI fails if a real user ID gets through anyway.

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p jes-reader-cli -- check corpus/sysout/JOB00896
cargo run -p jes-reader-cli -- check --rules my-rules.toml corpus/sysout   # house rules, see editors/vscode/README.md
```

### CLI

```sh
jes-reader check JOB00896                   # file:line:column: severity[code] message; exit code 1 on an error
jes-reader check --format sarif . > jes.sarif   # SARIF 2.1.0, for code scanning
jes-reader check --format json JOB00896
jes-reader summary JOB00868                 # per job: end, steps (program, outcome, CPU, elapsed), failures
jes-reader summary --format json JOB00868
```

Every command takes files or directories (one DD per file, as Zowe Explorer downloads them), `--rules <file>` (default: `.jesreader.toml` in the current directory, then in the home directory) and `--max-rc <n>`. Step programs come from the `JESJCL` of the same job, among the files or next to them.

```text
JRIDC8 JOB00868: ended, RC 8
  STEP     PROGRAM  OUTCOME  CPU     ELAPSED
  STEP005  IDCAMS   RC 4     0.00 s  0.00 s
  STEP010  IDCAMS   RC 8     0.00 s  0.00 s
  STEP020  IEFBR14  not run  0.00 s  0.00 s
  warning: Step STEP010: return code 8 (JOB00868/JES2/JESYSMSG.txt:19)
```

Corpus snapshots (`crates/jes-reader-core/tests/snapshots`) are updated with `INSTA_UPDATE=always cargo test -p jes-reader-core --test corpus`; review the diff before committing.

### VS Code extension

```sh
cd editors/vscode
npm ci
npm run build:server   # cargo build --release, copied into server/
npm run compile        # type check (tsc) and bundle (esbuild) into out/extension.js
npm run lint           # ESLint
```

Open `editors/vscode` in VS Code and run **JES Reader extension** (F5): a second window opens on `corpus/sysout`.

### Packaging and releases

`npm run package -- <target>` (in `editors/vscode`) builds `dist/jes-reader-<target>-<version>.vsix` with the server compiled for that platform. Targets: `win32-x64`, `win32-arm64`, `linux-x64`, `linux-arm64`, `darwin-x64`, `darwin-arm64`; without an argument, the current machine. Close any F5 development window first: it locks `server/`.

The `Package` workflow builds the VSIX and CLI archives for five platforms on every PR and push to `main` (run artifacts). To release:

1. Set the same version in `editors/vscode/package.json` and `Cargo.toml` — the workflow rejects a tag that does not match.
2. Push a `vX.Y.Z` tag.
3. The workflow creates the GitHub release with every VSIX and CLI archive attached, then publishes to the VS Code Marketplace when the repository variable `PUBLISH_MARKETPLACE` is `true`, to Open VSX when the `OVSX_PAT` secret exists, and the three crates to crates.io when the variable `PUBLISH_CRATES` is `true`. The jobs can be re-run: an existing release gets its assets replaced, and a crate version already on crates.io is skipped.

The version also appears in the `jes-reader-core` dependency of the root `Cargo.toml` (crates.io needs it); the workflow checks it too.

**VS Code Marketplace** publishing uses [Trusted Publishing](https://code.visualstudio.com/api/working-with-extensions/publishing-extension): GitHub proves the repository and workflow with an OIDC token (`vsce publish --oidc`, vsce 4), so no token is stored. Azure DevOps personal access tokens are not used: the global ones the Marketplace needed are being retired. Setup, once:

1. Create the `peariforme` publisher on [marketplace.visualstudio.com/manage](https://marketplace.visualstudio.com/manage) (the ID must match `publisher` in `editors/vscode/package.json`).
2. If Trusted Publishing cannot create a new extension, upload the first VSIX of each platform by hand from the same page.
3. On the publisher, under *Trusted Publishing*, add a policy for the GitHub repository `Peariforme/jes-reader` and the workflow `package.yml`.
4. In the repository settings, add the Actions variable `PUBLISH_MARKETPLACE` = `true`.

**crates.io** publishing uses [Trusted Publishing](https://crates.io/docs/trusted-publishing) as well: `rust-lang/crates-io-auth-action` exchanges GitHub's OIDC token for a short-lived publish token. Crates are published in dependency order: `jes-reader-core`, `jes-reader-lsp`, `jes-reader-cli`. Setup, once:

1. A crate must exist before Trusted Publishing can be set up on it: publish the first version by hand, from the tagged commit, with an API token from [crates.io/settings/tokens](https://crates.io/settings/tokens): `cargo login`, then `cargo publish --workspace`.
2. On each of the three crates, under *Settings → Trusted Publishing*, add the GitHub repository `Peariforme/jes-reader` and the workflow `package.yml`. Then revoke the API token.
3. In the repository settings, add the Actions variable `PUBLISH_CRATES` = `true`.

`cargo publish --workspace --dry-run` runs in CI so that the crates stay publishable.

**Open VSX**: `OVSX_PAT` is an open-vsx.org token, after creating the namespace once with `npx ovsx create-namespace peariforme`.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
