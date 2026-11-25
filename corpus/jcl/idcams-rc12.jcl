//JRIDC12  JOB (ACCT),'JES-READER IDCAMS12',CLASS=A,MSGCLASS=H,
//             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//*--------------------------------------------------------------------
//* CAS IDCAMS RC=12 : erreur de syntaxe dans les commandes.
//*   STEP010 : mot-cle inconnu sur LISTCAT    -> RC=12 (IDC32xxI)
//*   STEP020 : sans COND, s'execute quand meme -> RC=0
//* Attendu : $HASP395 ... ENDED - RC=0012.
//*--------------------------------------------------------------------
//STEP010  EXEC PGM=IDCAMS
//SYSPRINT DD SYSOUT=*
//SYSIN    DD *
  LISTCAT ENTRIES(JESREAD.INEXIST.RC12) JRMOTCLE
/*
//STEP020  EXEC PGM=IEFBR14
