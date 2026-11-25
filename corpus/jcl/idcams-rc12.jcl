//JRIDC12  JOB (ACCT),'JES-READER IDCAMS12',CLASS=A,MSGCLASS=H,
//             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//*--------------------------------------------------------------------
//* CASE IDCAMS RC=12: syntax error in the commands.
//*   STEP010: unknown keyword on LISTCAT       -> RC=12 (IDC32xxI)
//*   STEP020: no COND, runs anyway             -> RC=0
//* Expected: $HASP395 ... ENDED - RC=0012.
//*--------------------------------------------------------------------
//STEP010  EXEC PGM=IDCAMS
//SYSPRINT DD SYSOUT=*
//SYSIN    DD *
  LISTCAT ENTRIES(JESREAD.INEXIST.RC12) JRMOTCLE
/*
//STEP020  EXEC PGM=IEFBR14
