//JRLONG   JOB (ACCT),'JES-READER LONG',CLASS=A,MSGCLASS=H,
//             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//*--------------------------------------------------------------------
//* CASE LONG JOB: volume and structure, no abend.
//*   PRINT   : IDCAMS PRINT of a large member -> SYSPRINT of several
//*             dozen pages (ASA page breaks inside the DD)
//*   COPY    : IEBGENER to SYSOUT, DD written WITHOUT ASA characters
//*   RC4     : IDCAMS SET MAXCC=4                     -> RC=4
//*   SIOK    : IF (RC4.RC = 4) true                   -> runs
//*   SINON   : ELSE branch                            -> does not run
//*   APPEL1/APPEL2 : 2-step in-stream procedure called twice
//*             (<procstep> <step> names in the messages)
//*   FIN     : IEFBR14
//* Expected: $HASP395 ... ENDED - RC=0004, about ten steps.
//* If IHAPSA is not in SYS1.MACLIB, use another large member
//* (IEFUCBOB, IHASCB...).
//*--------------------------------------------------------------------
//JRPROC   PROC
//ETAPE1   EXEC PGM=IEFBR14
//ETAPE2   EXEC PGM=IDCAMS
//SYSPRINT DD SYSOUT=*
//SYSIN    DD DSN=&&CMDS,DISP=(OLD,PASS)
//         PEND
//*
//CMDS     EXEC PGM=IEBGENER
//SYSPRINT DD DUMMY
//SYSIN    DD DUMMY
//SYSUT1   DD *
  LISTCAT ENTRIES(JESREAD.INEXIST.PROC) NAME
  SET MAXCC=0
/*
//SYSUT2   DD DSN=&&CMDS,DISP=(NEW,PASS),
//             SPACE=(TRK,(1,1)),UNIT=SYSDA,
//             RECFM=FB,LRECL=80
//*
//PRINT    EXEC PGM=IDCAMS
//SYSPRINT DD SYSOUT=*
//SYSIN    DD *
  PRINT INDATASET('SYS1.MACLIB(IHAPSA)') CHARACTER COUNT(3000)
/*
//COPY     EXEC PGM=IEBGENER
//SYSPRINT DD SYSOUT=*
//SYSIN    DD DUMMY
//SYSUT1   DD DSN=SYS1.MACLIB(IHAPSA),DISP=SHR
//SYSUT2   DD SYSOUT=*
//RC4      EXEC PGM=IDCAMS
//SYSPRINT DD SYSOUT=*
//SYSIN    DD *
  SET MAXCC=4
/*
//TEST     IF (RC4.RC = 4) THEN
//SIOK     EXEC PGM=IEFBR14
//         ELSE
//SINON    EXEC PGM=IEFBR14
//         ENDIF
//APPEL1   EXEC JRPROC
//APPEL2   EXEC JRPROC
//FIN      EXEC PGM=IEFBR14
