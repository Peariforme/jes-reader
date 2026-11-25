//JRS0C7   JOB (ACCT),'JES-READER S0C7',CLASS=A,MSGCLASS=H,
//             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//*--------------------------------------------------------------------
//* CASE S0C7: an ADD on a COMP-3 field that holds 'ABC'
//* (X'C1C2C3', invalid sign 3) raises a data exception.
//*--------------------------------------------------------------------
//COBRUN   EXEC IGYWCLG
//COBOL.SYSIN DD *
       IDENTIFICATION DIVISION.
       PROGRAM-ID. ABEND0C7.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       01  WS-ALPHA            PIC X(3) VALUE 'ABC'.
       01  WS-PACKED REDEFINES WS-ALPHA
                               PIC S9(5) COMP-3.
       01  WS-TOTAL            PIC S9(5) COMP-3 VALUE 0.
       PROCEDURE DIVISION.
           ADD WS-PACKED TO WS-TOTAL
           DISPLAY 'TOTAL=' WS-TOTAL
           GOBACK.
/*
