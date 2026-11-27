# Contributing

Thank you for your interest in JES Reader. Bug reports, message patterns the analysis gets wrong, and pull requests are welcome.

## The one rule: no customer data

**Never commit, attach or paste a real sysout from your site**, not even in an issue. Sysouts hold data set names, user IDs and business data. Describe the message pattern instead, or rebuild a minimal example with the names changed.

The [corpus](https://github.com/Peariforme/jes-reader/tree/main/corpus) holds only sysouts produced on IBM Z Xplore from the jobs in `corpus/jcl`, or synthetic ones.

## Reporting a problem

[Open an issue](https://github.com/Peariforme/jes-reader/issues/new/choose) with:

- what you opened (Zowe Explorer document, downloaded job, BMC AMI DevX file…) and its DD;
- what JES Reader reported, and what you expected;
- the lines involved, anonymized;
- the versions of JES Reader, VS Code and Zowe Explorer.

## Development setup

You need a Rust toolchain (1.85 or later) and, for the VS Code extension, Node.js 22.

Once after cloning, enable the Git filter that anonymizes the corpus on `git add` (IBM Z Xplore user IDs become `Z99999`):

```sh
git config filter.anonymize.clean "sh scripts/anonymize.sh"
git config filter.anonymize.smudge cat
git config filter.anonymize.required true
```

Your working copy keeps the real values; only what is committed is anonymized. CI fails if a real user ID gets through anyway.

## Repository layout

| Path | Contents |
| --- | --- |
| `crates/jes-reader-core` | Parser, `Job → DD → step → message` model, rule engine, message catalog |
| `crates/jes-reader-lsp` | Language server (`lsp-server`, synchronous) |
| `crates/jes-reader-cli` | `jes-reader` CLI (`check` and `summary`) |
| `editors/vscode` | VS Code extension (thin client, bundled server) |
| `corpus/` | Test JCL and the sysouts they produce, no customer data |
| `docs/` | This documentation ([mdBook](https://rust-lang.github.io/mdBook/)) |

## Rust crates

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo run -p jes-reader-cli -- check corpus/sysout/JOB00896
```

Every corpus sysout has its expected analysis under version control as an [insta](https://insta.rs) snapshot (`crates/jes-reader-core/tests/snapshots`). After a change in the analysis, update them and review the diff before committing:

```sh
INSTA_UPDATE=always cargo test -p jes-reader-core --test corpus
```

The message catalog behind hovers and severities is `crates/jes-reader-core/catalog.toml`. Explanations are written in our own words: do not copy text from IBM manuals.

## VS Code extension

```sh
cd editors/vscode
npm ci
npm run build:server   # cargo build --release, copied into server/
npm run compile        # type check (tsc) and bundle (esbuild) into out/extension.js
npm run lint           # ESLint
```

Open `editors/vscode` in VS Code and run **JES Reader extension** (F5): a second window opens on `corpus/sysout`.

`npm run package -- <target>` builds `dist/jes-reader-<target>-<version>.vsix` with the server compiled for that platform. Targets: `win32-x64`, `win32-arm64`, `linux-x64`, `linux-arm64`, `darwin-x64`, `darwin-arm64`; without an argument, the current machine. Close any F5 development window first: it locks `server/`.

## Documentation

```sh
cargo install mdbook
mdbook serve docs --open
```

## Pull requests

- One topic per pull request, with a description of what changes for the user.
- Commits, comments and documentation in English.
- CI must pass: formatting, Clippy, tests on Linux and Windows, extension lint, and the corpus anonymization check.
- A change in the analysis comes with a corpus case or a unit test that shows it.

## License

By contributing, you agree that your contributions are licensed under the MIT and Apache-2.0 licenses, like the rest of the project.
