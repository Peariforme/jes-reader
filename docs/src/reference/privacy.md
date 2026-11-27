# Privacy and security

Sysouts hold data set names, user IDs and sometimes business data. JES Reader is built so that they never leave your workstation.

## What JES Reader does

- **It runs locally.** The analysis runs in a native process on your workstation, started by the editor.
- **It sends nothing over the network.** No telemetry, no usage statistics, no call to an online service. The language server and the CLI contain no network code.
- **It installs nothing on z/OS** and opens no connection to it. Only Zowe Explorer connects to z/OS, with your profiles and your credentials.
- **It only reads.** It never changes a sysout, a data set or a job.

## What goes through Zowe Explorer

In VS Code, two features ask Zowe Explorer to read from z/OS on your behalf:

- the `JESJCL` DD of a job opened from Zowe Explorer, fetched in the background for step programs and links to the JCL;
- the data set or member you Ctrl+click.

Both go through Zowe Explorer's file system, with the profile of the job (or `jesReader.zoweProfile`), exactly as if you had opened them from Zowe Explorer's tree.

## Links to IBM Docs

Hovers hold links to IBM Docs. Nothing is fetched until you click one; your browser then opens the IBM page.

## Offline and locked-down workstations

The extension works without internet access: install it from a VSIX (see [Installation](../getting-started/installation.md#offline-workstation)). It needs no admin rights and no runtime.

## Reporting a vulnerability

See the [security policy](https://github.com/Peariforme/jes-reader/security/policy).
