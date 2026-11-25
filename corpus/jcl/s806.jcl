//JRS806   JOB (ACCT),'JES-READER S806',CLASS=A,MSGCLASS=H,
//             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//*--------------------------------------------------------------------
//* CAS S806 : programme introuvable dans STEPLIB/JOBLIB/LNKLST.
//* Attendu : CSV003I REQUESTED MODULE JRNOPGM NOT FOUND, puis S806-04.
//*--------------------------------------------------------------------
//STEP010  EXEC PGM=JRNOPGM
//SYSPRINT DD SYSOUT=*
