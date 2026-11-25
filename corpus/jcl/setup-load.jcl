//JRSETUP  JOB (ACCT),'JES-READER SETUP',CLASS=A,MSGCLASS=H,
//             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//*--------------------------------------------------------------------
//* PREREQUISITE, submit once before the COBOL jobs
//* (s0c7, s0c4, s322, igz-open): the Z Xplore IGYWCLG procedure
//* stores the load module in &SYSUID..LOAD.
//* PDSE library (DSNTYPE=LIBRARY), RECFM=U: required for the
//* program objects the binder produces.
//*--------------------------------------------------------------------
//ALLOC    EXEC PGM=IEFBR14
//LOAD     DD DSN=&SYSUID..LOAD,DISP=(NEW,CATLG),
//             DSNTYPE=LIBRARY,RECFM=U,LRECL=0,BLKSIZE=32760,
//             SPACE=(TRK,(50,5)),UNIT=SYSDA
