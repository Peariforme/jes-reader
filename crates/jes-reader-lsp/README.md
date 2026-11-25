# jes-reader-lsp

Language server (LSP, over stdio) for z/OS JES2 sysouts: failures as diagnostics, job steps in the Outline, folding, Go to Definition from `JESYSMSG` and `JESMSGLG` to the expanded JCL, hovers on message IDs and abend codes.

```sh
cargo install jes-reader-lsp
```

This installs the `jes-reader-lsp` binary. In VS Code, install the [JES Reader extension](https://marketplace.visualstudio.com/items?itemName=peariforme.jes-reader) instead: it bundles the server. For another LSP client, start `jes-reader-lsp` with no arguments; initialization options:

```json
{ "maxReturnCode": 4, "rulesFile": "/path/to/.jesreader.toml" }
```

See the [repository](https://github.com/Peariforme/jes-reader) for the house rules format and the list of recognized files.

License: MIT or Apache-2.0, at your option.
