# Roadmap

## Vision and scope

A language server that turns a z/OS sysout into a navigable document: failures in the Problems panel, steps in the Outline, messages colored by severity. It runs entirely on the workstation; nothing is installed or opened on z/OS.

**Audience**: mainframe developers and operators on locked-down workstations, where VS Code is allowed but SSH and USS are not.

**In scope (v1)**

- JES2 sysouts downloaded as files, or opened directly from Zowe Explorer
- System DDs (JESMSGLG, JESJCL, JESYSMSG) and application DDs (SYSPRINT, SYSOUT, CEEDUMP read as text)
- Common IBM messages (IEF, IEC, IGD, $HASP, CEE, IGZ, DSN, DFS) and user-defined house rules

**Out of scope (v1)**

- Any connection to the mainframe: Zowe Explorer already does that
- JES3, to be reconsidered depending on the corpus
- Dump interpretation
- Reproducing IBM manuals (copyright): our own summaries and links to IBM Docs

Each phase ends with a verifiable exit criterion. The tool becomes useful day to day at the end of phase 1.

## Technical choices

A single Rust engine, exposed through a language server for editors and a CLI for CI.

| Component | Choice | Why |
| --- | --- | --- |
| Language | Rust | Single binary, no Node runtime to install for the server, fast parsing |
| Protocol | LSP (`lsp-server`) | Used by VS Code, Zed, Neovim and Eclipse |
| Highlighting | TextMate grammar (+ semantic tokens later) | TextMate works without the server; semantic tokens would carry the severity |
| VS Code extension | TypeScript + vscode-languageclient | Thin client, server binary bundled per platform |
| CLI | clap | `jes-reader check`, text output; JSON and SARIF later |
| Tests | insta (snapshots) | Every corpus sysout has its expected analysis under version control |

**Repository layout** (Cargo workspace)

- `crates/jes-reader-core`: parser, `Job → DD → step → message` model, rule engine
- `crates/jes-reader-lsp`: language server
- `crates/jes-reader-cli`: CLI
- `editors/vscode`: VS Code extension
- `editors/zed`: Zed extension (phase 4)
- `corpus/`: test sysouts and the JCL that produces them

## Phase 0 — Foundations and corpus

Goal: a repository that builds, and a corpus of real sysouts with no customer data.

- [x] Get IBM Z Xplore access; check that COBOL compiles and jobs can be submitted
- [x] Write jobs that fail on purpose: S0C7, S0C4, S806, S322, S013, JCL ERROR, data set not found, NOT CATLGD, RC 4/8/12 through IDCAMS, COBOL user abend (IGZ)
- [x] Retrieve each sysout through Zowe Explorer. Finding: Zowe reads and downloads spools one DD at a time (`<jobid>/<step>/<DD>.txt`), with no whole-spool file by default, so the parser works one DD at a time (see [FORMAT.md](FORMAT.md))
- [x] Cargo workspace and GitHub Actions CI (fmt, clippy, tests)
- [x] Pick the LSP library: `lsp-server`
- [x] Keep a [FORMAT.md](FORMAT.md): ASA column, line width, DD layout, page headers, JES2 header markers

**Exit**: at least 15 sysouts under version control and a green CI. **Status**: 13 sysouts; the last two jobs (`long.jcl`, `idcams-rc12.jcl`) are written but not submitted yet.

## Phase 1 — MVP

Goal: open a sysout in VS Code and see at once which step failed and why.

1. **Parser** (`jes-reader-core`): **the unit of parsing is the DD** (`parse_dd`), because Zowe Explorer opens spools one DD at a time; a whole spool would first be split into DDs. The DD kind comes from the file name or URI, otherwise from the content. `Job` is an optional aggregate of the open DDs with the same JES job ID (`JOB`, `TSU` or `STC`). Then: steps and their outcome (IEF start, end and return code messages), and messages by ID. The parser never fails: what it does not understand stays plain text.
2. **Language detection**: language ID `jes-sysout` by extension (`.sysout`, `.joblog`), by DD name and by content (JES2 header in the first lines), plus an "Open as Sysout" command. The client also attaches to IBM Z Open Editor's language IDs (`spool`, `jesmsglg`, `jesjcl`, assigned by file name): when that extension is installed it keeps its basic TextMate highlighting and we add diagnostics and the Outline on top. Z Open Editor has no language server for spools (checked on 6.9.0). Nothing is copied from its grammar (IBM property).
3. **TextMate grammar**: message IDs, return codes, abend codes, data set names, JCL statements.
4. **Language server**: Outline of steps, folding, diagnostics (abends and JCL errors as errors, RC > 4 as warnings, configurable threshold).
5. **VS Code extension**: thin client, bundled binary.
6. **Tests**: one snapshot per corpus sysout.

**Exit**: on the whole corpus, the failing step is at the top of the Problems panel and the Outline is right.

**Status**: done. All six items are implemented, and the exit criterion was checked by hand in VS Code (Outline, Problems, language detection). Semantic tokens are not done: the TextMate grammar already colors the known error messages. Findings:

- Severity cannot be read from the suffix: `IEF453I`, `IEF472I`, `IEFA107I` end in `I`. The rule engine (`analysis.rs`) and the grammar share a list of known messages that must be kept in sync.
- Severity comes from the steps (abend, RC, not run), then from that list, then from the `E`/`S`/`W` suffix.
- `IDI0078E` (Fault Analyzer cannot write its history file) is reported as an error although it is not about the job: a typical case for the house rules of phase 3.

## Phase 2 — Zowe Explorer and hovers

Goal: the tool is used on everyday jobs, without downloading anything.

- [x] Identify the URI scheme of spools opened by Zowe Explorer (`zowe-jobs:`) and attach the extension to it. Checked: a job opened directly from Zowe Explorer is analysed (diagnostics, Outline).
- [x] Links: statement numbers in JESYSMSG to the matching line of JESJCL (Go to Definition)
- [x] Navigation: from a step in JESYSMSG or in the JESMSGLG step table to its EXEC in JESJCL, procedure steps included (`COBRUN.GO` → the expanded `GO` EXEC)
- [x] Cross-DD links for Zowe documents: the extension opens just the JESJCL DD in the background through Zowe's file system (`….JES2.JESJCL.3`), without downloading the whole spool. Checked against a live Zowe connection. Downloaded jobs: the server reads `JESJCL.txt` next to the file.
- [x] Hovers: our own TOML catalog (`crates/jes-reader-core/catalog.toml`, 51 messages and 20 abends to start) with a link to IBM Docs for all the others. The same catalog makes diagnostics readable (title, reason code meaning), decides the severity of messages whose suffix is misleading, and links each failed step to the messages that caused it
- [x] Clickable data set names opening in Zowe Explorer: in documents opened from Zowe, data set names link to `zowe-ds:/<profile>/<DSN>[/<member>]` (Zowe Explorer's file system), the profile being the job's. Temporary and spool data sets, symbolic names, DD overrides and dump text are left out

**Exit**: a job opened from Zowe Explorer is analysed without any download.

## Phase 3 — Summaries, house rules, CLI

Goal: the tool adapts to each site's conventions and is also useful outside the editor.

- [ ] Code lens per step: name, program, return code, CPU and elapsed time when the end-of-step messages give them
- [x] House rules in a `.jesreader.toml` (setting, workspace or home directory): message ID pattern and optional text → severity or `ignore`, return code tolerated per step name pattern, notes. Reloaded on save; an invalid file is reported and ignored. Same file for the CLI (`--rules`)
- [ ] House rules by program (`PGM=IDCAMS` → RC 4 accepted): needs the step program from JESJCL
- [ ] CLI: `jes-reader summary`, JSON and SARIF output for `check`

**Exit**: a rules file of about ten lines is enough to adapt the tool to an application.

## Phase 4 — Distribution

Goal: one-click install at home, and offline install on a locked-down workstation.

- [x] Cross-compiled releases through GitHub Actions (Windows x64, Linux x64 and arm64, macOS x64 and arm64), published on `v*` tags
- [ ] Publication on the VS Code Marketplace and Open VSX (workflow ready, needs the `VSCE_PAT` and `OVSX_PAT` secrets)
- [ ] Platform-specific VSIX installed offline and tested on a Windows workstation without admin rights
- [ ] Zed extension: language declaration and LSP launch. Zed does not use TextMate, so highlighting needs a tree-sitter grammar
- [ ] README with an animated capture, announcement on the Open Mainframe Project Slack

**Exit**: an offline VSIX works on a Windows workstation without admin rights.

## Risks and open questions

| Risk | Consequence | Mitigation |
| --- | --- | --- |
| Format varies with the export tool | Fragile parsing | Multi-source corpus, tolerant parser that degrades instead of failing |
| Corpus limited to Z Xplore | Production messages missing | House rules, collecting message patterns (never data) from users |
| Confidentiality of customer sysouts | Real cases cannot be shared | No customer sysout in the repository, cases rebuilt as synthetic data |
| IBM documentation copyright | Thin hovers | Our own summaries, links to IBM Docs |
| Zowe Explorer URI scheme changes | Broken integration | Integration test on a pinned version |

**Open questions**

- JES3 in v1 or later
- Whole-spool files: which tools produce them, and how DDs are separated
