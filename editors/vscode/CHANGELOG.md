# Changelog

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
