//JRNOTCAT JOB (ACCT),'JES-READER NOTCATLG',CLASS=A,MSGCLASS=H,
//             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//*--------------------------------------------------------------------
//* CASE NOT CATLGD: the same data set created twice in a row.
//*   STEP005: clean-up of a previous run (MOD,DELETE)
//*   STEP010: create and catalog                       -> RC=0
//*   STEP020: same name with (NEW,CATLG)
//* Expected, depending on whether the volume is SMS-managed:
//*   - SMS     : IGD17101I DUPLICATE DATA SET NAME, step not run
//*   - non-SMS : IEF287I ... NOT CATLGD 2
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
