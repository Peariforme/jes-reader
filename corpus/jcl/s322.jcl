//JRS322   JOB (ACCT),'JES-READER S322',CLASS=A,MSGCLASS=H,
//             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//*--------------------------------------------------------------------
//* CASE S322: endless loop, CPU limit of step GO set to 1 second.
//* Expected: IEF450I ... ABEND=S322, step GO.
//*--------------------------------------------------------------------
//COBRUN   EXEC IGYWCLG,TIME.GO=(0,1)
//COBOL.SYSIN DD *
       IDENTIFICATION DIVISION.
       PROGRAM-ID. ABEND322.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       01  WS-N                PIC 9(9) COMP VALUE 0.
       01  WS-STOP             PIC X VALUE 'N'.
       PROCEDURE DIVISION.
           PERFORM UNTIL WS-STOP = 'Y'
               ADD 1 TO WS-N
           END-PERFORM
           GOBACK.
/*
