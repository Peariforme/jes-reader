# jes-reader-lsp

Language server (LSP, over stdio) for z/OS JES2 sysouts: failures as diagnostics, job steps in the Outline, folding, Go to Definition from `JESYSMSG` and `JESMSGLG` to the expanded JCL, hovers on message IDs and abend codes.

```sh
cargo install jes-reader-lsp
```

This installs the `jes-reader-lsp` binary. In VS Code, install the [JES Reader extension](https://marketplace.visualstudio.com/items?itemName=peariforme.jes-reader) instead: it bundles the server. For another LSP client, start `jes-reader-lsp` with no arguments; initialization options:

```json
{ "maxReturnCode": 4, "rulesFile": "/path/to/.jesreader.toml" }
```

Configuration for Neovim and Helix, house rules and recognized files: see the [documentation](https://peariforme.github.io/jes-reader/guide/other-editors.html).

License: MIT or Apache-2.0, at your option.
