# Changelog

## 0.2.1

- An icon, and screenshots on this page.
- A documentation site: [peariforme.github.io/jes-reader](https://peariforme.github.io/jes-reader/), with the installation on an offline workstation, the house rules reference, the command line in CI, and the configuration of other editors.

No change in the analysis.

## 0.2.0

**Steps and house rules**

- Step summaries: a code lens above each step of `JESYSMSG` with its program, return code or abend, CPU time and elapsed time. The Outline shows each step's program.
- House rules by program: `[[steps]]` rules take a `program` pattern (`PGM=` of the EXEC statement, read in `JESJCL`), alone or with `name`.
- Step tables of `JESMSGLG` with another layout (a `JOBNAME` column first, `CPU` instead of `TCB`) are read: their columns come from the header line.

**Whole spools and BMC AMI DevX**

- Whole spools: a file holding every DD of a job, one after the other, is cut into its DDs, with one Outline entry per DD and links to the `JESJCL` of the same file.
- BMC AMI DevX: the `.jes` file of the Job Explorer is recognized (its name gives the job name and ID) and cut into its DDs. DevX writes `JESJCL` one column to the right of `JESMSGLG`: each DD is now read with its own layout (in place, one column to the right, or without the ASA column).
- Messages that programs write to the console (`+WLM644E` in `JESMSGLG`) are read as messages.
- Lines starting with a non-ASCII character (a badly converted carriage-control byte) are read with a space in its place.

**Zowe Explorer and data set links**

- A partitioned data set opens in Zowe Explorer's Data Sets view instead of an error. It is recognized before opening (Zowe Explorer's file system says it is a directory) and from the "actually a directory" error.
- The new `jesReader.zoweProfile` setting enables data set links in documents that do not come from Zowe Explorer (BMC AMI DevX, downloaded files).
- Links from `JESYSMSG` to `JESJCL` no longer need the job ID in the document name: it is read from the content, and the job name is used when there is none.
- Fewer requests to Zowe Explorer: the `JESJCL` of a Zowe spool is fetched in the background at most once per session, never when it is already open, and no longer on hovers or code lenses (Go to Definition may fetch it again). Each fetch could make Zowe Explorer ask for credentials.
- When Zowe Explorer refuses a link because its credentials prompt was cancelled, JES Reader says so and what to do, instead of showing Zowe's raw error.

**Editor**

- Message IDs are colored by the severity of the analysis (semantic highlighting), house rules included.
- Folding in `JESJCL`: an `EXEC` folds up to the next step (its procedure and DDs included), a procedure `EXEC` up to the next `EXEC`, the `JOB` card the whole job. It used to stop at the first DD.
- 19 more messages in the catalog: IDCAMS, Language Environment, COBOL, Db2 and IMS.

**Performance and diagnosis**

- Lighter data set links: each link carries only its position, and its target is built when it is followed. Links are also kept for each version of a document. On a large sysout, coming back to the editor no longer makes VS Code copy and parse hundreds of kilobytes of links.
- A **JES Reader** output channel logs the server start, each analysis (lines, DDs, column offsets, diagnostics, time), slow requests, data set links and background `JESJCL` fetches. New setting `jesReader.trace.server` traces every exchange with the language server.
- The language server no longer stops when a request fails or panics: it answers with an error, logs it, and keeps the analysis of the open documents.
- Smaller, faster package: the client is bundled with esbuild into a single file, and the VSIX no longer ships `node_modules` (10 files instead of 326).

## 0.1.0

First release.

**Analysis**

- Diagnostics for abends, JCL errors, return codes above a configurable threshold and steps that did not run. Severity does not rely on the message suffix alone: `IEF453I` (JCL ERROR) and `IEF472I` (abend) both end in `I`.
- Readable diagnostics: catalog title, reason code meaning and the full message text, continuation lines included (`OPEN failed (S013-18: member not found in the partitioned data set): … SYS1.MACLIB(JRNOMBR)`).
- A failed or skipped step is linked to the messages that caused it (related information), and back.
- House rules in a `.jesreader.toml` file: message severity (or `ignore`), return code tolerated per step, notes added to diagnostics. Found through the `jesReader.rulesFile` setting, the workspace root or the home directory, and reloaded on save.

**Navigation**

- Outline of steps (`JESMSGLG`, `JESYSMSG`) and of the expanded JCL (`JESJCL`).
- Go to Definition from `JESYSMSG` and `JESMSGLG` to `JESJCL`: statement numbers of conversion messages, and steps to their `EXEC` statement (procedure steps included). For Zowe Explorer documents, the `JESJCL` DD is opened in the background.
- Link targets (statement numbers, step names) are underlined; hovering one shows the JCL statement it leads to.
- Data set names in documents opened from Zowe Explorer link to the data set or member, opened through Zowe Explorer with the profile of the job.
- Folding of steps, statements and multi-line messages.

**Explanations**

- Hovers on message IDs and abend codes from our own catalog (51 messages, 20 abends), with links to the z/OS 3.2 message pages and to an IBM Docs search.

**Editor**

- TextMate grammar for message IDs (including Language Environment's critical `C` suffix), abend codes, return codes, data set names and JCL.
- Works on documents opened from Zowe Explorer, on downloaded jobs, and alongside IBM Z Open Editor.
