//JRIDC8   JOB (ACCT),'JES-READER IDCAMS',CLASS=A,MSGCLASS=H,
//             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//*--------------------------------------------------------------------
//* CASE IDCAMS: three return codes in one job.
//*   STEP005: LISTCAT of a missing entry       -> RC=4
//*   STEP010: DELETE of a missing entry        -> RC=8 (IDC3012I)
//*   STEP020: COND=(8,LE) true, step bypassed  -> NOT EXECUTED
//*--------------------------------------------------------------------
//STEP005  EXEC PGM=IDCAMS
//SYSPRINT DD SYSOUT=*
//SYSIN    DD *
  LISTCAT ENTRIES(JESREAD.INEXIST.LISTCAT)
/*
//STEP010  EXEC PGM=IDCAMS
//SYSPRINT DD SYSOUT=*
//SYSIN    DD *
  DELETE JESREAD.INEXIST.DELETE
/*
//STEP020  EXEC PGM=IEFBR14,COND=(8,LE)
