# Changelog

## Unreleased

- Folding in `JESJCL`: an `EXEC` folds up to the next step (its procedure and DDs included), a procedure `EXEC` up to the next `EXEC`, the `JOB` card the whole job. It used to stop at the first DD.
- Step tables of `JESMSGLG` with another layout (a `JOBNAME` column first, `CPU` instead of `TCB`) are read: their columns come from the header line.
- BMC AMI DevX: the `.jes` file of the Job Explorer (every DD of a job, one after the other) is recognized and cut into its DDs. Messages that programs write to the console (`+WLM644E` in `JESMSGLG`) are read as messages. Sysouts without the ASA column are read too.
- Data set links: a partitioned data set opens in Zowe Explorer's Data Sets view instead of an error. The new `jesReader.zoweProfile` setting enables links in documents that do not come from Zowe Explorer.
- Links from `JESYSMSG` to `JESJCL` no longer need the job ID in the document name: it is read from the content, and the job name is used when there is none.
- Whole spools: a file holding every DD of a job (BMC AMI DevX JES Explorer) is cut into its DDs, with one Outline entry per DD and links to the `JESJCL` of the same file.
- Message IDs are colored by the severity of the analysis (semantic highlighting), house rules included.
- 19 more messages in the catalog: IDCAMS, Language Environment, COBOL, Db2 and IMS.
- Step summaries: a code lens above each step of `JESYSMSG` with its program, return code or abend, CPU time and elapsed time.
- The Outline shows each step's program.
- House rules by program: `[[steps]]` rules take a `program` pattern (`PGM=` of the EXEC statement, read in `JESJCL`), alone or with `name`.
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
