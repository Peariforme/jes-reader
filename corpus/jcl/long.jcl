//JRLONG   JOB (ACCT),'JES-READER LONG',CLASS=A,MSGCLASS=H,
//             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//*--------------------------------------------------------------------
//* CAS JOB LONG : volume et structure, sans abend.
//*   PRINT   : IDCAMS PRINT d'un gros membre -> SYSPRINT de plusieurs
//*             dizaines de pages (sauts de page ASA au fil de la DD)
//*   COPY    : IEBGENER vers SYSOUT, DD ecrite SANS caractere ASA
//*   RC4     : IDCAMS SET MAXCC=4                     -> RC=4
//*   SIOK    : IF (RC4.RC = 4) vrai                   -> execute
//*   SINON   : branche ELSE                           -> non execute
//*   APPEL1/APPEL2 : procedure in-stream de 2 steps appelee 2 fois
//*             (noms <procstep> <step> dans les messages)
//*   FIN     : IEFBR14
//* Attendu : $HASP395 ... ENDED - RC=0004, une dizaine de steps.
//* Si IHAPSA est absent de SYS1.MACLIB, prendre un autre gros
//* membre (IEFUCBOB, IHASCB...).
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
