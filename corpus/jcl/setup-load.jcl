//JRSETUP  JOB (ACCT),'JES-READER SETUP',CLASS=A,MSGCLASS=H,
//             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//*--------------------------------------------------------------------
//* PREREQUIS, a soumettre une seule fois avant les jobs COBOL
//* (s0c7, s0c4, s322, igz-open) : la procedure IGYWCLG de Z Xplore
//* range le module dans &SYSUID..LOAD.
//* Bibliotheque PDSE (DSNTYPE=LIBRARY), RECFM=U : obligatoire pour
//* les program objects produits par le binder.
//*--------------------------------------------------------------------
//ALLOC    EXEC PGM=IEFBR14
//LOAD     DD DSN=&SYSUID..LOAD,DISP=(NEW,CATLG),
//             DSNTYPE=LIBRARY,RECFM=U,LRECL=0,BLKSIZE=32760,
//             SPACE=(TRK,(50,5)),UNIT=SYSDA
