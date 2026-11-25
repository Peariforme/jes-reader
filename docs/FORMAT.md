# JES2 sysout format

Observation notes, extended as the corpus grows. Every statement cites the corpus file that shows it. Columns are numbered from 1, ASA column included.

The sysouts are listed in [corpus/README.md](../corpus/README.md). All come from IBM Z Xplore (z/OS, JES2), downloaded with Zowe Explorer on Windows.

## Retrieval through Zowe Explorer

- Zowe Explorer reads spools **one DD at a time**: each DD is a separate document; there is no "whole spool" file by default.
- Downloading a job produces `<jobid>/<step>/<DD>.txt`. System DDs go under `JES2/` (`JOB00823/JES2/JESMSGLG.txt`), application DDs under the step name (`JOB00823/VALIDATE/SYSTSPRT.txt`).
- For a step that comes from a procedure, **the procedure step comes first**, then the job step: `<jobid>/<procstep>/<step>/<DD>.txt` (`JOB00896/GO/COBRUN/SYSOUT.txt`, where `GO` is the `IGYWCLG` step and `COBRUN` the job step).
- Zowe sometimes adds a `(1)` suffix to the file name with no apparent duplicate in the folder (`JOB00896/LKED/COBRUN/SYSPRINT(1).txt`). The DD name is the file name without that suffix.
- Installed products add their own DDs: IBM Fault Analyzer writes `IDIREPRT` on abends (`JOB00901/STEP010/IDIREPRT.txt`, 2,448 lines).
- Opened without downloading, the document is named `<jobname>.<jobid>.<step>.<DD>.<n>` (`JRIDC12.JOB00922.JES2.JESMSGLG.2`), where `n` is the DD number in the spool. For a TSO session the job name is the user ID (`Z99999.TSU00812.JES2.JESYSMSG.4`).
- The trailing number is the JES data set ID. JES2 numbers the system DDs JESMSGLG 2, JESJCL 3, JESYSMSG 4 (`JRIDC12.JOB00922.JES2.JESMSGLG.2`, `Z99999.TSU00812.JES2.JESYSMSG.4`); the extension relies on it to open the JESJCL of a Zowe document.
- Consequence for the parser: **the unit of parsing is the DD**. The file name (or URI) gives the DD; the content is the fallback (see "Detection").
- Whole spool in a single file: _not observed yet_ (separator between DDs unknown).

## ASA carriage control (column 1)

- **Present in JES2 DDs and most application DDs, even when downloaded one by one.**
- **Exception**: the Fault Analyzer report `IDIREPRT` has none (`JOB00901/STEP010/IDIREPRT.txt`). For application DDs the parser detects it: a DD has ASA characters if every non-empty line starts with ` `, `0`, `-`, `+` or `1`.
- `1` new page: first line of JESMSGLG (`JOB00823/JES2/JESMSGLG.txt`) and of `SYSTSPRT` (`1READY`, `JOB00823/VALIDATE/SYSTSPRT.txt`)
- `0` double spacing: second line of JESMSGLG, and `0------ JES2 JOB STATISTICS ------`
- `-` triple spacing: JES2 statistics lines at the end of JESMSGLG
- ` ` single spacing: everything else; JESJCL and JESYSMSG only have spaces in column 1
- `+` overprint: _not observed yet_
- Impact on the parser: ignore column 1 when analysing, but keep it in positions sent to the editor (editor columns include the ASA character).

## Line width and line endings

- Longest line observed: **133** (132 + ASA), in the JESMSGLG step table (`JOB00823/JES2/JESMSGLG.txt`).
- **Trailing spaces are trimmed** in every file: line lengths vary, no line ends with a space.
- **CRLF** line endings in files downloaded on Windows, last line included. Whether z/OSMF or Zowe adds them is _not verified_; the parser accepts LF and CRLF.
- The corpus is stored without conversion (`.gitattributes`).

## JESMSGLG

```
1                   J E S 2  J O B  L O G  --  S Y S T E M  S 0 W 1  --  N O D E  S V S C J E S 2
0
 04.12.28 JOB00823 ---- WEDNESDAY, 25 NOV 2026 ----
 04.12.28 JOB00823  IRR010I  USERID Z99999   IS ASSIGNED TO THIS JOB.
```

- Line 1: banner, system (`S0W1`) and JES2 node (`SVSCJES2`).
- Date line: `---- <DAY>, <dd> <MMM> <yyyy> ----`.
- Messages: time `hh.mm.ss` (columns 2-9), JES job ID (columns 11-18), two spaces, message ID from column 21.
- **JES job ID**: `JOB` prefix for batch, `TSU` for a TSO session (`TSU00812/JES2/JESMSGLG.txt`), `STC` for a started task (_not observed yet_). The parser accepts all three.
- Step summary table, prefixed with `-` after the job ID (`JOB00823/JES2/JESMSGLG.txt`):

  ```
   04.12.31 JOB00823  -STEPNAME PROCSTEP    RC   EXCP   CONN       TCB       SRB  CLOCK          SERV  WORKLOAD  PAGE  SWAP   VIO SWAPS
   04.12.31 JOB00823  -VALIDATE             00    205      8       .00       .00     .0            23  BATCH        0     0     0     0
   04.12.31 JOB00823  -@VSC1    ENDED.  NAME-                     TOTAL TCB CPU TIME=      .00 TOTAL ELAPSED TIME=    .0
  ```

  It gives each step's return code without JESYSMSG. The `RC` column is columns 39-44: a system abend has a leading asterisk (`*S0C7`, `JOB00896`), a user abend does not (`U4038`, `JOB00885`). A step that did not run shows `FLUSH`, shifted under `PROCSTEP` (`JOB00889/JES2/JESMSGLG.txt`: `-STEP010           FLUSH`). The exact layout depends on the installation (IEFACTRT exit): _to be confirmed on other systems_.
- End of job, followed by the `JES2 JOB STATISTICS` block. `$HASP395` comes in three forms:
  - `ENDED - RC=0008`: highest step return code (`JOB00868`)
  - `ENDED - ABEND=S0C4` or `ABEND=U4038` (`JOB00893`, `JOB00885`)
  - `ENDED` alone, after `IEF453I <job> - JOB FAILED - JCL ERROR` (`JOB00889`, `JOB00890`)
- Abend: `IEF450I <job> <procstep> <step> - ABEND=S000 U4038 REASON=00000001` in JESMSGLG (`JOB00885`).
- **Job still running**: JESMSGLG stops after `$HASP373 … STARTED`, with no `$HASP395` and no statistics (`TSU00812/JES2/JESMSGLG.txt`). A missing end is not an error.

## JESJCL

```
        1 //@VSC1     JOB ,MSGLEVEL=(0,0),CLASS=7                                 JOB00823
```

- Statement number right-aligned in columns 2-9, a space, then the statement from column 11. Careful: in JESYSMSG the number is one column further right (columns 2-10).
- Continuation lines: blank number (`TSU00812/JES2/JESJCL.txt`, `//             REGION=65536K`).
- Statements from a procedure: `XX` instead of `//` (`XXIZUFPROC PROC …`); `X/` for a procedure statement overridden by the job.
- Symbol substitution: `IEFC653I SUBSTITUTION JCL - …` from column 11, with no number.
- The JOB card carries the JES job ID at the end of the line (columns 83-90).
- These statement numbers are the targets of the `STMT NO.` column in JESYSMSG.

## JESYSMSG

```
  STMT NO. MESSAGE
         2 IEFC001I PROCEDURE ZXP$$CHK WAS EXPANDED USING SYSTEM LIBRARY VENDOR.PROCLIB
 IEF142I @VSC1 VALIDATE - STEP WAS EXECUTED - COND CODE 0000
```

- First line: `STMT NO. MESSAGE` header.
- JCL conversion messages: statement number in columns 2-10, message ID from column 12.
- Execution messages: message ID in column 2.
- Continuation lines: usually indented (column 10 or more), without a message ID (`CPU: …`, `VIRT: …`, `SWA=ABOVE,…`).
- **Pitfall**: some continuations start in column 2, like a new message. Examples: `IGD17101I` then ` NOT DEFINED BECAUSE DUPLICATE NAME EXISTS IN CATALOG` (`JOB00890`), `IEC141I 013-18,…` then ` SYS1.MACLIB(JRNOMBR)` (`JOB00901`). A line in column 2 only starts a new message if it begins with a valid message ID; otherwise it continues the previous one.
- Step names in messages: `<job> <step>` for a plain step, `<job> <procstep> <step>` for a procedure step (`IEF142I JRS0C7 COBOL COBRUN - …`).
- Step timestamps: `IEF373I STEP/<step>/START yyyyddd.hhmm` and `IEF032I STEP/<step>/STOP yyyyddd.hhmm` (Julian date).
- End of job: `IEF375I JOB/<job>/START …` and `IEF033I JOB/<job>/STOP …`.

## Detection (content, when the file name says nothing)

| DD | Reliable pattern |
| --- | --- |
| JESMSGLG | line 1: `J E S 2  J O B  L O G` |
| JESJCL | line 1: number `1` in column 9, `//` in column 11, `JOB` card |
| JESYSMSG | line 1: `STMT NO. MESSAGE` |
| Other DDs | no general pattern: file name or URI required |

## Key messages

| Message | DD | Meaning | Example |
| --- | --- | --- | --- |
| `$HASP373` | JESMSGLG | Job started | `$HASP373 @VSC1    STARTED - INIT 1    - CLASS 7        - SYS S0W1` |
| `$HASP395` | JESMSGLG | Job ended, with highest return code | `$HASP395 @VSC1    ENDED - RC=0000` |
| `IEF142I` | JESYSMSG | Step ended with return code | `IEF142I @VSC1 VALIDATE - STEP WAS EXECUTED - COND CODE 0000` |
| `IEF373I` / `IEF032I` | JESYSMSG | Step start / stop | `IEF373I STEP/VALIDATE/START 2026329.0412` |
| `IEF375I` / `IEF033I` | JESYSMSG | Job start / stop | `IEF375I  JOB/@VSC1   /START 2026329.0412` |
| `IEFC001I` | JESYSMSG | Procedure expanded | `2 IEFC001I PROCEDURE ZXP$$CHK WAS EXPANDED …` |
| `IEF472I` | JESYSMSG | Abnormal end (abend) | `IEF472I JRS0C7 GO COBRUN - COMPLETION CODE - SYSTEM=0C7 USER=0000 REASON=00000007` |
| `IEF450I` | JESMSGLG | Abnormal end (abend) | `IEF450I JRIGZ GO COBRUN - ABEND=S000 U4038 REASON=00000001` |
| `IEFA107I` | JESYSMSG | Data set not found at allocation (not `IEF212I` on Z Xplore) | `IEFA107I JRJCLERR STEP010 SYSUT1 - DATA SET Z99999.JESREAD.INEXIST NOT FOUND` |
| `IEF202I` | JESYSMSG | Step not run (COND) | `IEF202I JRIDC8 STEP020 - STEP WAS NOT RUN BECAUSE OF CONDITION CODES` |
| `IEF272I` | JESYSMSG | Step not executed (after `IEF202I` or an allocation error) | `IEF272I JRIDC8 STEP020 - STEP WAS NOT EXECUTED.` |
| `IEF453I` | JESMSGLG | JCL ERROR | `IEF453I JRJCLERR - JOB FAILED - JCL ERROR` |
| `IGD17101I` | JESYSMSG | Duplicate data set (SMS), continued in column 2 | `IGD17101I DATA SET Z99999.JESREAD.DUP` |
| `IEC141I` | JESYSMSG | OPEN abend (013) | `IEC141I 013-18,IGG0191B,JRS013,STEP010,SYSUT1,1001,VTMVSC,` |
| `CSV003I` | JESYSMSG | Module not found (S806) | `CSV003I REQUESTED MODULE JRNOPGM  NOT FOUND` |
| `IGZ0035S` | SYSOUT (step GO) | COBOL OPEN failed, followed by U4038 | `IGZ0035S There was an unsuccessful OPEN or CLOSE of file INFILE …` |
