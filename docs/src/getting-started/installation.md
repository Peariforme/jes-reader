# Installation

## VS Code

Install **JES Reader** from the [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=peariforme.jes-reader), or search for `JES Reader` in the Extensions view (Ctrl+Shift+X).

The extension bundles the language server as a native binary: there is no runtime to install. It needs VS Code 1.90 or later, on Windows x64, Linux x64 or arm64, or macOS x64 or arm64.

To open spools directly from z/OS, also install [Zowe Explorer](https://marketplace.visualstudio.com/items?itemName=Zowe.vscode-extension-for-zowe). JES Reader works without it on files you have downloaded.

### Offline workstation

Many mainframe workstations have no access to the Marketplace. Install from a VSIX instead; no admin rights are needed:

1. On a machine with internet access, download the VSIX for your platform from the [latest GitHub release](https://github.com/Peariforme/jes-reader/releases/latest), for example `jes-reader-win32-x64-0.2.0.vsix`.
2. Copy it to the workstation.
3. In VS Code, run **Extensions: Install from VSIX…** from the Command Palette (Ctrl+Shift+P) and pick the file.

| Platform | VSIX |
| --- | --- |
| Windows x64 | `jes-reader-win32-x64-<version>.vsix` |
| Linux x64 | `jes-reader-linux-x64-<version>.vsix` |
| Linux arm64 | `jes-reader-linux-arm64-<version>.vsix` |
| macOS Intel | `jes-reader-darwin-x64-<version>.vsix` |
| macOS Apple silicon | `jes-reader-darwin-arm64-<version>.vsix` |

## Command line

With a Rust toolchain:

```sh
cargo install jes-reader-cli
```

This installs the `jes-reader` binary. Without Rust, download the archive for your platform from the [latest release](https://github.com/Peariforme/jes-reader/releases/latest) (`jes-reader-cli-<platform>-<version>.zip` or `.tar.gz`) and put `jes-reader` on your `PATH`. See [Command line](../guide/cli.md).

## Language server for another editor

```sh
cargo install jes-reader-lsp
```

See [Other editors](../guide/other-editors.md) for Neovim and Helix.

## Next step

[Open your first sysout](first-sysout.md).
