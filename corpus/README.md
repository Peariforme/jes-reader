# Corpus

Test sysouts and the JCL that produces them.

**Hard rule: no customer sysout, no real data.** Everything here is produced on IBM Z Xplore from the jobs in `jcl/`, or rebuilt by hand as synthetic data.

Z Xplore user IDs (`Z` followed by 5 digits) are replaced with `Z99999` on `git add` by the `anonymize` filter (see the root README to enable it). CI fails if a real user ID gets through anyway.

## Layout

- `jcl/`: jobs that fail on purpose, one case per file
- `sysout/`: retrieved sysouts, **in the folder layout Zowe Explorer produces when downloading a job**, not renamed. This is what the parser gets in real life:

  ```
  sysout/<jobid>/JES2/JESMSGLG.txt               system DDs
  sysout/<jobid>/JES2/JESJCL.txt
  sysout/<jobid>/JES2/JESYSMSG.txt
  sysout/<jobid>/<step>/<DD>.txt                  application DDs
  sysout/<jobid>/<procstep>/<step>/<DD>.txt       application DDs of a procedure step
  ```

  A whole spool downloaded as a single file, if a tool allows it, goes to `sysout/<jobid>.spool.txt`.

Files in this folder are never converted (line endings included, see `.gitattributes`). Do not reformat them.

## Retrieved sysouts

| Folder | Case | JCL | Outcome | Why it is useful |
| --- | --- | --- | --- | --- |
| `sysout/JOB00823` | Z Xplore validation | (provided by Z Xplore) | RC=0 | Application DD `SYSTSPRT` |
| `sysout/TSU00812` | TSO session (z/OSMF) | | session still active | `TSU` prefix, no end of job |
| `sysout/JOB00868` | IDCAMS RC=8 | `jcl/idcams-rc8.jcl` | RC 4, RC 8, STEP020 `IEF202I` | `$HASP395 ENDED - RC=0008` |
| `sysout/JOB00869` | JCL ERROR in a procedure | `jcl/igz-open.jcl` without `<userid>.LOAD` | COBOL step RC=0, LKED `IEFA107I` on `SYSLMOD`, `IEF453I` | JCL ERROR after a step that ran; numbered conversion warnings (`IEF655I`, `IEF686I`) |
| `sysout/JOB00884` | LOAD library allocation | `jcl/setup-load.jcl` | RC=0 | Minimal IEFBR14 job |
| `sysout/JOB00885` | IGZ | `jcl/igz-open.jcl` | `IGZ0035S`, U4038 | Procedure steps, `SYSOUT` DD of the GO step |
| `sysout/JOB00889` | JCL ERROR | `jcl/jcl-error.jcl` | `IEFA107I`, `IEF453I` | `FLUSH` in the step table |
| `sysout/JOB00890` | NOT CATLGD | `jcl/not-catlgd.jcl` | `IGD17101I` (SMS), JCL ERROR | Message continued in column 2 |
| `sysout/JOB00893` | S0C4 | `jcl/s0c4.jcl` | S0C4 in step GO | |
| `sysout/JOB00896` | S0C7 | `jcl/s0c7.jcl` | S0C7 in step GO | |
| `sysout/JOB00901` | S013 | `jcl/s013.jcl` | `IEC141I 013-18` | Fault Analyzer report (`IDIREPRT`) |
| `sysout/JOB00903` | S322 | `jcl/s322.jcl` | S322 in step GO | |
| `sysout/JOB00905` | S806 | `jcl/s806.jcl` | `CSV003I`, S806-04 | Fault Analyzer report (`IDIREPRT`) |

On Z Xplore, S0C7 and S0C4 end as system abends; Language Environment does not turn them into U4038.

## Cases still to collect

| Case | JCL and expected outcome |
| --- | --- |
| IDCAMS RC=12 | `jcl/idcams-rc12.jcl`: unknown keyword, `$HASP395 … RC=0012` |
| Long job | `jcl/long.jcl`: about ten steps, `IF/THEN/ELSE`, an in-stream procedure called twice, a SYSPRINT of several dozen pages, a DD written without ASA characters (IEBGENER) |

Before the COBOL jobs (s0c7, s0c4, s322, igz-open), submit `jcl/setup-load.jcl` once: on Z Xplore the `IGYWCLG` procedure stores the load module in `<userid>.LOAD`, and the job fails with a JCL ERROR if that library does not exist.

On Z Xplore, replace `CLASS=A` with the class you are allowed to use (`CLASS=7` on `JOB00823`) and check the name of the COBOL compile procedure (`IGYWCLG` by default). For each job, download the spool with Zowe Explorer's download action (right-click the job) and drop it as is into `sysout/`.
