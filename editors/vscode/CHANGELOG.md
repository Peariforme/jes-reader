# Changelog

## Unreleased

- Go to Definition from `JESYSMSG` and `JESMSGLG` to `JESJCL`: statement numbers of conversion messages, and steps to their `EXEC` statement (procedure steps included). For Zowe Explorer documents, the `JESJCL` DD is opened in the background.
- Link targets (statement numbers, step names) are underlined; hovering one shows the JCL statement it leads to.
- Hovers on message IDs and abend codes from our own catalog (51 messages, 20 abends), with a link to IBM Docs.
- Readable diagnostics: catalog title, reason code meaning and the full message text, continuation lines included (`OPEN failed (S013-18: member not found in the partitioned data set): … SYS1.MACLIB(JRNOMBR)`).
- A failed or skipped step is linked to the messages that caused it (related information), and back.
- Messages ending in `C` (Language Environment critical, `CEE3250C`) are recognized and reported as errors.

## 0.1.0

First release.

- Diagnostics for abends, JCL errors, return codes above a configurable threshold and steps that did not run.
- Outline of steps (`JESMSGLG`, `JESYSMSG`) and of the expanded JCL (`JESJCL`).
- Folding of steps, statements and multi-line messages.
- TextMate grammar for message IDs, abend codes, return codes, data set names and JCL.
- Works on documents opened from Zowe Explorer, on downloaded jobs, and alongside IBM Z Open Editor.
