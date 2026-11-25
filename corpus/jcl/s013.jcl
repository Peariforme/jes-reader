//JRS013   JOB (ACCT),'JES-READER S013',CLASS=A,MSGCLASS=H,
//             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//*--------------------------------------------------------------------
//* CAS S013 : membre absent d'un PDS existant. L'allocation passe,
//* l'OPEN echoue.
//* Attendu : IEC141I 013-18, abend S013 au step STEP010.
//*--------------------------------------------------------------------
//STEP010  EXEC PGM=IEBGENER
//SYSPRINT DD SYSOUT=*
//SYSIN    DD DUMMY
//SYSUT1   DD DSN=SYS1.MACLIB(JRNOMBR),DISP=SHR
//SYSUT2   DD SYSOUT=*
