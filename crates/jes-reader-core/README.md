# jes-reader-core

Parsing and analysis of z/OS JES2 sysouts, one DD at a time (`JESMSGLG`, `JESJCL`, `JESYSMSG`, `SYSPRINT`…), as Zowe Explorer opens or downloads them.

- Model: `Job → DD → step → message`, with JCL statements for `JESJCL`.
- Findings: abends and JCL errors, return codes above a threshold, steps that did not run. Severity does not rely on the message suffix alone (`IEF453I` and `IEF472I` both end in `I`).
- A catalog of common messages and abend codes, in our own words, with links to IBM Docs.
- House rules (`.jesreader.toml`) to adapt the analysis to a site.

The parser never fails: whatever it does not understand stays plain text.

```rust
use jes_reader_core::{analyze, parse_file, Options};

let text = " IEF472I JRS0C7 GO COBRUN - COMPLETION CODE - SYSTEM=0C7 USER=0000 REASON=00000007\n";
let dd = parse_file("JOB00896/JES2/JESYSMSG.txt", text);
let findings = analyze(&dd, &Options::default());
assert_eq!(findings[0].code, "S0C7");
```

Used by the [`jes-reader-lsp`](https://crates.io/crates/jes-reader-lsp) language server and the [`jes-reader-cli`](https://crates.io/crates/jes-reader-cli) command line. See the [repository](https://github.com/Peariforme/jes-reader) for the VS Code extension and the sysout format notes.

License: MIT or Apache-2.0, at your option.
