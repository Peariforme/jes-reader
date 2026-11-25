# jes-reader-cli

Command-line checker for z/OS JES2 sysouts, for scripts and CI.

```sh
cargo install jes-reader-cli
jes-reader check JOB00896          # file:line:column: severity[code] message
```

This installs the `jes-reader` binary. It takes files or directories, one DD per file, as Zowe Explorer downloads them (`JOB00896/JES2/JESYSMSG.txt`…), and exits with code 1 when it finds an error (abend, JCL error…). Options: `--max-rc <n>`, the return code above which a step is reported (default 4), and `--rules <file>`, the house rules (default: `.jesreader.toml` in the current directory, then in the home directory).

Prebuilt binaries for Windows, Linux and macOS are attached to each [GitHub release](https://github.com/Peariforme/jes-reader/releases).

License: MIT or Apache-2.0, at your option.
