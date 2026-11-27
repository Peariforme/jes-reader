# Recognized files

JES Reader analyses a document when it recognizes it as a sysout:

| Source | Example |
| --- | --- |
| Documents opened from Zowe Explorer | `JRS0C7.JOB00896.JES2.JESYSMSG.4` |
| Jobs downloaded with Zowe Explorer | `JOB00896/JES2/JESYSMSG.txt`, also `TSU` and `STC` jobs |
| Files with a sysout extension | `.sysout`, `.joblog` |
| BMC AMI DevX Job Explorer | `<jobname>_<jobid>.jes` |
| Any file whose first line is a JES2 header | `J E S 2  J O B  L O G`, `STMT NO. MESSAGE`, a numbered `JOB` card |

For anything else, run **JES Reader: Open as Sysout** from the Command Palette.

## One DD at a time

Zowe Explorer opens each DD of a job as its own document. JES Reader analyses each one on its own, so you never need to download the whole spool. The DD comes from the document name, or from its content when the name says nothing.

## Whole spools

A file holding every DD of a job, one after the other, is cut into its DDs:

- the Outline has one entry per DD;
- each DD is analysed as if it were opened on its own, with its line numbers kept;
- links go to the `JESJCL` in the same file.

The output of the steps (`SYSPRINT`, `SYSOUT`…) stays one block for now: its DDs are not separated or named yet.

## BMC AMI DevX

The Job Explorer of BMC AMI DevX opens a job as one `.jes` file holding all its DDs. JES Reader cuts it into its DDs, reads the job name and ID from the file name, and handles DevX's layout (`JESJCL` one column to the right of `JESMSGLG`, step tables with a `JOBNAME` column).

For data set links, set `jesReader.zoweProfile` to the Zowe Explorer profile of the same system.

## IBM Z Open Editor

IBM Z Open Editor already highlights some spool files (`spool`, `jesmsglg` and `jesjcl` languages). JES Reader attaches to those languages too: Z Open Editor keeps its highlighting, and JES Reader adds diagnostics, the Outline and folding.
