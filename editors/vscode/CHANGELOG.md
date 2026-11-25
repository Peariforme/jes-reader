# Changelog

## Unreleased

- Go to Definition from `JESYSMSG` and `JESMSGLG` to `JESJCL`: statement numbers of conversion messages, and steps to their `EXEC` statement (procedure steps included). For Zowe Explorer documents, the `JESJCL` DD is opened in the background.

## 0.1.0

First release.

- Diagnostics for abends, JCL errors, return codes above a configurable threshold and steps that did not run.
- Outline of steps (`JESMSGLG`, `JESYSMSG`) and of the expanded JCL (`JESJCL`).
- Folding of steps, statements and multi-line messages.
- TextMate grammar for message IDs, abend codes, return codes, data set names and JCL.
- Works on documents opened from Zowe Explorer, on downloaded jobs, and alongside IBM Z Open Editor.
