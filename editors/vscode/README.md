# JES Reader

Read z/OS JES2 sysouts the way you read code: failures in the **Problems** panel, job steps in the **Outline**, messages colored by severity.

JES Reader works on the spool files you open from [Zowe Explorer](https://marketplace.visualstudio.com/items?itemName=Zowe.vscode-extension-for-zowe) and on sysouts you have downloaded. Everything runs locally: nothing is installed or started on z/OS.

## Features

- **Problems panel**: abends (`S0C7`, `S806`, `U4038`…), JCL errors, data sets not found and module-not-found messages are errors; steps that ended above the return-code threshold are warnings; steps that did not run are information. Severity does not rely on the message suffix alone — `IEF453I` (JCL ERROR) and `IEF472I` (abend) both end in `I`.
- **Outline**: in `JESMSGLG` and `JESYSMSG`, every step with its outcome (`RC 0`, `abend S0C7`, `not run`) and its program, procedure steps included (`COBRUN.GO`); in `JESJCL`, a `JOB → EXEC → procedure EXEC → DD` tree.
- **Step summaries** (code lenses) above each step of `JESYSMSG`: name, program, return code or abend, CPU time and elapsed time, for example `STEP010 · IDCAMS · RC 8 · CPU 0.01 s · elapsed < 1 min`. The program comes from `JESJCL` (fetched in the background for Zowe documents, read next to downloaded files). Elapsed time in `JESYSMSG` is only known to the minute. Turn code lenses off with the `editor.codeLens` setting.
- **Go to Definition** (Ctrl+click or F12) from `JESYSMSG` and `JESMSGLG` to the expanded JCL in `JESJCL`: on a statement number (`11 IEF655I …`) it opens statement 11; on a step's outcome line (`IEF142I`, `IEF472I`, a row of the step table) it opens the step's `EXEC`, inside the procedure for procedure steps. Link targets are underlined, and hovering one shows the JCL statement it leads to. For documents opened from Zowe Explorer, JES Reader fetches just the `JESJCL` DD in the background.
- **Data set links**: Ctrl+click a data set name (`SYS1.MACLIB(JRNOMBR)`, `ISP.SISPEXEC`…) to open it, or the member, with Zowe Explorer. A partitioned data set opens in Zowe Explorer's Data Sets view. Documents opened from Zowe Explorer use the profile of their job; for the others (BMC AMI DevX, downloaded files), set `jesReader.zoweProfile`.
- **Folding** of steps, JCL statements and multi-line messages.
- **Hovers on messages and abend codes**: a short explanation in our own words, the meaning of the reason code when known (`S013-18`: member not found), what to check, and links to IBM Docs: the message page of the z/OS 3.2 documentation for z/OS messages (`IEF`, `IEC`, `IGD`, `IEA`, `CSV`, `IDC`, `ICH`, `IRR`, `$HASP`, `CEE`), and a search for the others. Abend codes are recognized wherever they appear (`SYSTEM=0C7`, `*S0C7`, `ABEND=U4038`, `ABEND806-04`).
- **Cause and effect**: in the Problems panel, a failed step points to the messages that explain it (for example `IEC141I` for an `S013`), and each of those messages points back to the step.
- **Syntax highlighting** of message IDs, abend codes, return codes above 4, data set names and JCL statements.
- **One DD at a time**: Zowe Explorer opens each DD as its own document; JES Reader analyses each one on its own, so you never need to download the whole spool.
- **Whole spools** too: a file holding every DD of a job, one after the other (as saved by BMC AMI DevX JES Explorer), is cut into its DDs. The Outline has one entry per DD, and links go to the `JESJCL` in the same file. The output of the steps stays one block for now.
- **Colors that follow the analysis**: message IDs are colored by the severity JES Reader reports, house rules included (semantic highlighting): a message you silence with a rule is no longer red.

## Which files are recognized

- Documents opened from Zowe Explorer (`<job>.<jobid>.<step>.<DD>.<n>`)
- Jobs downloaded with Zowe Explorer (`JOBnnnnn/…/<DD>.txt`, also `TSU` and `STC`)
- `.sysout` and `.joblog` files
- Any file whose first line is a JES2 header, a whole spool included when it starts with JESMSGLG (`J E S 2  J O B  L O G`, `STMT NO. MESSAGE`, or a numbered `JOB` card)

For anything else, run **JES Reader: Open as Sysout** from the Command Palette.

### With BMC AMI DevX

The Job Explorer of BMC AMI DevX opens a job as one `.jes` file holding all its DDs. JES Reader cuts it into its DDs: one Outline entry per DD, the steps under `JESYSMSG`, and links from `JESYSMSG` to the `JESJCL` of the same file. For data set links, set `jesReader.zoweProfile` to the Zowe Explorer profile of the same system.

### With IBM Z Open Editor

IBM Z Open Editor already highlights some spool files (`spool`, `jesmsglg` and `jesjcl` languages). JES Reader attaches to those languages too: Z Open Editor keeps its highlighting, and JES Reader adds diagnostics, the Outline and folding.

## House rules

A `.jesreader.toml` file adapts the analysis to your site: silence a message that is not about your jobs, tolerate a return code for a step, or raise a message to an error. JES Reader looks for it in this order: the `jesReader.rulesFile` setting, the root of the workspace, your home directory. Changes are picked up as soon as you save the file.

```toml
# Return code above which a step is reported (default 4).
max_return_code = 4

# Messages: severity "error", "warning", "info" or "ignore". `*` matches anything.
[[messages]]
id = "IDI0078E"
severity = "ignore"
note = "Fault Analyzer history file, not about the job"

[[messages]]
id = "IEF287I"
text = "NOT CATLGD 2"      # only when the message text contains this
severity = "error"

# Steps: return code tolerated, by step name (`step` or `step.procstep`, `*` allowed)…
[[steps]]
name = "COBRUN.LKED"
max_return_code = 8
note = "link-edit warnings are expected"

# …or by program (`PGM=` of the EXEC statement), or both.
[[steps]]
program = "IDCAMS"
max_return_code = 8
note = "RC 8: nothing to delete"
```

Rules apply in file order: the first matching rule wins. A step rule needs a `name`, a `program` or both; with both, both must match. Program rules need the job's `JESJCL`: JES Reader fetches it in the background for documents opened from Zowe Explorer, and reads `JESJCL.txt` next to downloaded files. The `note` is added to the diagnostic. An invalid file is reported in a notification and ignored.

## Settings

| Setting | Default | Description |
| --- | --- | --- |
| `jesReader.maxReturnCode` | `4` | A step whose return code is above this value is reported as a warning. |
| `jesReader.rulesFile` | `""` | House rules file. Empty: `.jesreader.toml` at the root of the workspace, then in your home directory. |
| `jesReader.serverPath` | `""` | Path to a `jes-reader-lsp` binary. Empty: use the one bundled with the extension. |
| `jesReader.zoweProfile` | `""` | Zowe Explorer profile for the data set links of documents that do not come from Zowe Explorer (BMC AMI DevX, downloaded files). Empty: links only in Zowe documents. |

## Commands

- **JES Reader: Open as Sysout** — treat the active document as a sysout.
- **JES Reader: Restart Server**

## Supported platforms

Windows x64, Linux x64 and arm64, macOS x64 and arm64. The language server is a native binary bundled in the platform-specific package; no runtime needs to be installed.

## Known limitations

- JES2 only; JES3 sysouts are not recognized.
- In a whole spool, the output of the steps (`SYSPRINT`, `SYSOUT`…) is one block: its DDs are not separated or named yet.
- Some product messages are reported by their suffix even when they are not about your job (for example Fault Analyzer's `IDI0078E`): silence them with a house rule.
- A step that runs a program loaded from a previous step (`PGM=*.LKED.SYSLMOD`) shows that reference as its program.

### Zowe Explorer credentials

A data set link, or the `JESJCL` that JES Reader fetches in the background for a Zowe spool, goes through Zowe Explorer, which may ask for your credentials. If you cancel that prompt, Zowe Explorer may refuse every read from then on (`AuthCancelledError: User cancelled previous authentication`), even after you connect from its tree; run **Developer: Reload Window** to start again. To avoid it, connect from Zowe Explorer before following links.

### Diagnosing slowness

The **JES Reader** output channel (View → Output) logs the start of the language server, the analysis of each document (lines, DDs found with their lines and column offset, diagnostics, time), requests slower than 20 ms, the data set links computed for each document, and the `JESJCL` fetched in the background. Set its level to Trace (gear icon of the Output view) to see more. Set `jesReader.trace.server` to `messages` to log every exchange with the language server and its duration.

All extensions share one extension host: when links or hovers come late although JES Reader's log shows short times, another extension is busy. *Developer: Show Running Extensions* and *Developer: Start Extension Host Profile* show which one.

## Feedback

Issues and suggestions: [github.com/Peariforme/jes-reader](https://github.com/Peariforme/jes-reader/issues). Please never attach a real customer sysout — describe the message pattern instead.

## License

MIT or Apache-2.0, at your option.
