# JES Reader

Read z/OS JES2 sysouts the way you read code: failures in the **Problems** panel, job steps in the **Outline**, messages colored by severity.

JES Reader works on the spool files you open from [Zowe Explorer](https://marketplace.visualstudio.com/items?itemName=Zowe.vscode-extension-for-zowe) and on sysouts you have downloaded. Everything runs locally: nothing is installed or started on z/OS.

## Features

- **Problems panel**: abends (`S0C7`, `S806`, `U4038`…), JCL errors, data sets not found and module-not-found messages are errors; steps that ended above the return-code threshold are warnings; steps that did not run are information. Severity does not rely on the message suffix alone — `IEF453I` (JCL ERROR) and `IEF472I` (abend) both end in `I`.
- **Outline**: in `JESMSGLG` and `JESYSMSG`, every step with its outcome (`RC 0`, `abend S0C7`, `not run`), procedure steps included (`COBRUN.GO`); in `JESJCL`, a `JOB → EXEC → procedure EXEC → DD` tree.
- **Go to Definition** (Ctrl+click or F12) from `JESYSMSG` and `JESMSGLG` to the expanded JCL in `JESJCL`: on a statement number (`11 IEF655I …`) it opens statement 11; on a step's outcome line (`IEF142I`, `IEF472I`, a row of the step table) it opens the step's `EXEC`, inside the procedure for procedure steps. Link targets are underlined, and hovering one shows the JCL statement it leads to. For documents opened from Zowe Explorer, JES Reader fetches just the `JESJCL` DD in the background.
- **Data set links** in documents opened from Zowe Explorer: Ctrl+click a data set name (`SYS1.MACLIB(JRNOMBR)`, `ISP.SISPEXEC`…) to open it, or the member, with Zowe Explorer, using the profile of the job.
- **Folding** of steps, JCL statements and multi-line messages.
- **Hovers on messages and abend codes**: a short explanation in our own words, the meaning of the reason code when known (`S013-18`: member not found), what to check, and links to IBM Docs: the message page of the z/OS 3.2 documentation for z/OS messages (`IEF`, `IEC`, `IGD`, `IEA`, `CSV`, `IDC`, `ICH`, `IRR`, `$HASP`, `CEE`), and a search for the others. Abend codes are recognized wherever they appear (`SYSTEM=0C7`, `*S0C7`, `ABEND=U4038`, `ABEND806-04`).
- **Cause and effect**: in the Problems panel, a failed step points to the messages that explain it (for example `IEC141I` for an `S013`), and each of those messages points back to the step.
- **Syntax highlighting** of message IDs, abend codes, return codes above 4, data set names and JCL statements.
- **One DD at a time**: Zowe Explorer opens each DD as its own document; JES Reader analyses each one on its own, so you never need to download the whole spool.

## Which files are recognized

- Documents opened from Zowe Explorer (`<job>.<jobid>.<step>.<DD>.<n>`)
- Jobs downloaded with Zowe Explorer (`JOBnnnnn/…/<DD>.txt`, also `TSU` and `STC`)
- `.sysout` and `.joblog` files
- Any file whose first line is a JES2 header (`J E S 2  J O B  L O G`, `STMT NO. MESSAGE`, or a numbered `JOB` card)

For anything else, run **JES Reader: Open as Sysout** from the Command Palette.

### With IBM Z Open Editor

IBM Z Open Editor already highlights some spool files (`spool`, `jesmsglg` and `jesjcl` languages). JES Reader attaches to those languages too: Z Open Editor keeps its highlighting, and JES Reader adds diagnostics, the Outline and folding.

## Settings

| Setting | Default | Description |
| --- | --- | --- |
| `jesReader.maxReturnCode` | `4` | A step whose return code is above this value is reported as a warning. |
| `jesReader.serverPath` | `""` | Path to a `jes-reader-lsp` binary. Empty: use the one bundled with the extension. |

## Commands

- **JES Reader: Open as Sysout** — treat the active document as a sysout.
- **JES Reader: Restart Server**

## Supported platforms

Windows x64, Linux x64 and arm64, macOS x64 and arm64. The language server is a native binary bundled in the platform-specific package; no runtime needs to be installed.

## Known limitations

- JES2 only; JES3 sysouts are not recognized.
- A whole spool saved as a single file is not split into DDs yet.
- Some product messages are reported by their suffix even when they are not about your job (for example Fault Analyzer's `IDI0078E`). Configurable house rules are planned.

## Feedback

Issues and suggestions: [github.com/Peariforme/jes-reader](https://github.com/Peariforme/jes-reader/issues). Please never attach a real customer sysout — describe the message pattern instead.

## License

MIT or Apache-2.0, at your option.
