# Settings and commands

## Settings

| Setting | Default | Description |
| --- | --- | --- |
| `jesReader.maxReturnCode` | `4` | A step whose return code is above this value is reported as a warning. A `max_return_code` in the [house rules](../guide/house-rules.md) takes precedence. |
| `jesReader.rulesFile` | `""` | House rules file. Empty: `.jesreader.toml` at the root of the workspace, then in your home directory. |
| `jesReader.serverPath` | `""` | Path to a `jes-reader-lsp` binary. Empty: use the one bundled with the extension. |
| `jesReader.zoweProfile` | `""` | Zowe Explorer profile for the data set links of documents that do not come from Zowe Explorer (BMC AMI DevX, downloaded files). Empty: links only in Zowe documents. |
| `jesReader.trace.server` | `off` | Logs the exchanges with the language server in the **JES Reader** output channel: `off`, `messages` or `verbose`. |

Related VS Code settings:

- `editor.codeLens`: turns the step summaries off.
- `editor.semanticHighlighting.enabled`: with `false`, message IDs are colored by the grammar only, not by the severity JES Reader reports.

## Commands

| Command | Effect |
| --- | --- |
| **JES Reader: Open as Sysout** | Treat the active document as a sysout, when JES Reader did not recognize it. |
| **JES Reader: Restart Server** | Restart the language server. Changing a `jesReader` setting restarts it on its own. |
