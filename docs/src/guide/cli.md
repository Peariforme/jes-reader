# Command line

The `jes-reader` command runs the same analysis as the editor, for scripts and CI pipelines. See [Installation](../getting-started/installation.md#command-line).

Every command takes files or directories: one DD per file, as Zowe Explorer and the Zowe CLI download them (`JOB00896/JES2/JESYSMSG.txt`…), or a whole spool in one file (BMC AMI DevX `.jes`). Step programs come from the `JESJCL` of the same job, among the files or next to them.

## `jes-reader check`

Reports failures, one per line:

```console
$ jes-reader check JOB00896
JOB00896/GO/COBRUN/SYSOUT.txt:1:2: error[CEE3207S] Data exception (0C7): The system detected a data exception (System Completion Code=0C7). …
JOB00896/JES2/JESMSGLG.txt:13:1: error[S0C7] Step COBRUN.GO: abend S0C7 (Data exception)
JOB00896/JES2/JESYSMSG.txt:131:1: error[S0C7] Step COBRUN.GO: abend S0C7 (Data exception), reason 00000007
JOB00896/JES2/JESYSMSG.txt:3:12: warning[IEF655I] Invalid DSNAME: INVALID DSNAME SPECIFIED WHEN SYSIN OR SYSOUT SPECIFIED
JOB00896/JES2/JESYSMSG.txt:4:12: warning[IEF686I] DDNAME reference not resolved: DDNAME REFERRED TO ON DDNAME KEYWORD IN PRIOR STEP WAS NOT RESOLVED
6 file(s), 3 error(s), 2 warning(s)
```

It exits with code 1 when it finds an error, 0 otherwise.

| Option | Effect |
| --- | --- |
| `--format text` | `file:line:column: severity[code] message` (default) |
| `--format json` | One JSON document with every finding, its position and related messages |
| `--format sarif` | SARIF 2.1.0, for code scanning tools (GitHub, Azure DevOps…) |
| `--max-rc <n>` | Return code above which a step is reported (default 4, or the rules file's `max_return_code`) |
| `--rules <file>` | [House rules](house-rules.md). Default: `.jesreader.toml` in the current directory, then in the home directory |

## `jes-reader summary`

Prints each job: how it ended, then each step with its program, outcome, CPU and elapsed time, then its errors and warnings.

```console
$ jes-reader summary JOB00896
JRS0C7 JOB00896: ended, abend S0C7
  STEP          PROGRAM         OUTCOME              CPU     ELAPSED
  COBRUN.COBOL  IGYCRCTL        RC 0                 0.04 s  0.00 s
  COBRUN.LKED   IEWBLINK        RC 0                 0.01 s  0.00 s
  COBRUN.GO     *.LKED.SYSLMOD  abend S0C7 reason 7  0.00 s  0.00 s
  error: Step COBRUN.GO: abend S0C7 (Data exception), reason 00000007 (JOB00896/JES2/JESYSMSG.txt:131)
  …
```

A failure that appears in several DDs is reported once per job. `--format json` gives the same content for scripts; `--max-rc` and `--rules` work as for `check`.

## In a CI pipeline

Submit a job, download its output with the [Zowe CLI](https://docs.zowe.org/stable/user-guide/cli-using-usingcli), then check it:

```sh
jobid=$(zowe zos-jobs submit data-set "MY.JCL(NIGHTLY)" --wait-for-output --rff jobid --rft string)
zowe zos-jobs download output "$jobid" --directory sysout
jes-reader summary sysout
jes-reader check sysout      # fails the pipeline on an abend or a JCL error
```

### GitHub code scanning

With `--format sarif`, findings show up in the *Security → Code scanning* tab and on pull requests:

```yaml
- name: Check the job output
  run: jes-reader check --format sarif sysout > jes-reader.sarif
  continue-on-error: true
- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: jes-reader.sarif
```
