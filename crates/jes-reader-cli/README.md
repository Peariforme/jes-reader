# jes-reader-cli

Command-line checker for z/OS JES2 sysouts, for scripts and CI.

```sh
cargo install jes-reader-cli
jes-reader check JOB00896                       # file:line:column: severity[code] message
jes-reader check --format sarif . > jes.sarif   # SARIF 2.1.0, for code scanning
jes-reader summary JOB00868                     # per job: end, steps, failures
```

This installs the `jes-reader` binary. It takes files or directories: one DD per file, as Zowe Explorer downloads them (`JOB00896/JES2/JESYSMSG.txt`…), or a whole spool in one file (BMC AMI DevX `.jes`).

- `check` reports abends, JCL errors, return codes above a threshold and steps that did not run, as text, JSON (`--format json`) or SARIF (`--format sarif`). It exits with code 1 when it finds an error.
- `summary` prints each job: its end, then each step with its program, outcome, CPU and elapsed time, then its errors and warnings; `--format json` for scripts.

Options: `--max-rc <n>`, the return code above which a step is reported (default 4), and `--rules <file>`, the house rules (default: `.jesreader.toml` in the current directory, then in the home directory).

Prebuilt binaries for Windows, Linux and macOS are attached to each [GitHub release](https://github.com/Peariforme/jes-reader/releases).

License: MIT or Apache-2.0, at your option.
