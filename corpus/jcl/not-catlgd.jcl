//JRNOTCAT JOB (ACCT),'JES-READER NOTCATLG',CLASS=A,MSGCLASS=H,
//             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//*--------------------------------------------------------------------
//* CAS NOT CATLGD : deux creations successives du meme dataset.
//*   STEP005 : nettoyage d'une execution precedente (MOD,DELETE)
//*   STEP010 : creation et catalogage                 -> RC=0
//*   STEP020 : meme nom en (NEW,CATLG)
//* Attendu, selon que le volume est gere par SMS ou non :
//*   - SMS     : IGD17101I DUPLICATE DATA SET NAME, step non execute
//*   - non SMS : IEF287I ... NOT CATLGD 2
//*--------------------------------------------------------------------
//STEP005  EXEC PGM=IEFBR14
//DEL      DD DSN=&SYSUID..JESREAD.DUP,DISP=(MOD,DELETE),
//             SPACE=(TRK,(1,1)),UNIT=SYSDA
//STEP010  EXEC PGM=IEFBR14
//NEW1     DD DSN=&SYSUID..JESREAD.DUP,DISP=(NEW,CATLG),
//             SPACE=(TRK,(1,1)),UNIT=SYSDA,
//             RECFM=FB,LRECL=80
//STEP020  EXEC PGM=IEFBR14
//NEW2     DD DSN=&SYSUID..JESREAD.DUP,DISP=(NEW,CATLG),
//             SPACE=(TRK,(1,1)),UNIT=SYSDA,
//             RECFM=FB,LRECL=80
