//JRS806   JOB (ACCT),'JES-READER S806',CLASS=A,MSGCLASS=H,
//             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//*--------------------------------------------------------------------
//* CASE S806: program not found in STEPLIB/JOBLIB/LNKLST.
//* Expected: CSV003I REQUESTED MODULE JRNOPGM NOT FOUND, then S806-04.
//*--------------------------------------------------------------------
//STEP010  EXEC PGM=JRNOPGM
//SYSPRINT DD SYSOUT=*
