//JRJCLERR JOB (ACCT),'JES-READER JCLERR',CLASS=A,MSGCLASS=H,
//             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//*--------------------------------------------------------------------
//* CASE JCL ERROR: DISP=SHR on a data set that does not exist.
//* Expected: IEFA107I ... NOT FOUND, then IEF453I JCL ERROR.
//*--------------------------------------------------------------------
//STEP010  EXEC PGM=IEBGENER
//SYSPRINT DD SYSOUT=*
//SYSIN    DD DUMMY
//SYSUT1   DD DSN=&SYSUID..JESREAD.INEXIST,DISP=SHR
//SYSUT2   DD SYSOUT=*
