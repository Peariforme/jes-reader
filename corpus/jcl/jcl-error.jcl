//JRJCLERR JOB (ACCT),'JES-READER JCLERR',CLASS=A,MSGCLASS=H,
//             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//*--------------------------------------------------------------------
//* CAS JCL ERROR : DISP=SHR sur un dataset qui n'existe pas.
//* Attendu : IEF212I ... DATA SET NOT FOUND, puis JOB NOT RUN.
//*--------------------------------------------------------------------
//STEP010  EXEC PGM=IEBGENER
//SYSPRINT DD SYSOUT=*
//SYSIN    DD DUMMY
//SYSUT1   DD DSN=&SYSUID..JESREAD.INEXIST,DISP=SHR
//SYSUT2   DD SYSOUT=*
