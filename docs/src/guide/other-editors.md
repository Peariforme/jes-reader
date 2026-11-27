# Other editors

The analysis lives in a language server, `jes-reader-lsp`, that any LSP client can start. Install it with:

```sh
cargo install jes-reader-lsp
```

It talks over stdio and takes no arguments. Initialization options:

```json
{ "maxReturnCode": 4, "rulesFile": "/path/to/.jesreader.toml" }
```

Without `rulesFile`, it looks for `.jesreader.toml` at the root of the workspace, then in your home directory.

Outside VS Code you get diagnostics, the Outline (document symbols), folding, Go to Definition to the JCL, hovers, code lenses and semantic tokens, depending on what your editor supports. Zowe Explorer integration (fetching `JESJCL` in the background, data set links) is specific to VS Code; for downloaded jobs, the server reads `JESJCL.txt` next to the file on its own.

## Neovim

With Neovim 0.11 or later, in your `init.lua`:

```lua
vim.filetype.add({
  extension = { sysout = 'jessysout', joblog = 'jessysout', jes = 'jessysout' },
  pattern = { ['.*/JOB%d+/.*%.txt'] = 'jessysout' },
})

vim.lsp.config('jes_reader', {
  cmd = { 'jes-reader-lsp' },
  filetypes = { 'jessysout' },
  init_options = { maxReturnCode = 4 },
})
vim.lsp.enable('jes_reader')
```

## Helix

In `languages.toml`:

```toml
[language-server.jes-reader]
command = "jes-reader-lsp"
config = { maxReturnCode = 4 }

[[language]]
name = "jes-sysout"
scope = "source.jes-sysout"
file-types = ["sysout", "joblog", "jes"]
roots = []
language-servers = ["jes-reader"]
```

## Other clients

Any client that can start a stdio language server for a file type works the same way: start `jes-reader-lsp`, attach it to the files you want analysed, and pass the options above. Contributions of tested configurations for other editors are welcome.
