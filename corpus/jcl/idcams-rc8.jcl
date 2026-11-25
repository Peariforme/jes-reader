//JRIDC8   JOB (ACCT),'JES-READER IDCAMS',CLASS=A,MSGCLASS=H,
//             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//*--------------------------------------------------------------------
//* CAS IDCAMS : trois codes retour dans un meme job.
//*   STEP005 : LISTCAT d'une entree absente     -> RC=4
//*   STEP010 : DELETE d'une entree absente      -> RC=8 (IDC3012I)
//*   STEP020 : COND=(8,LE) vrai, step saute     -> NOT EXECUTED
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
