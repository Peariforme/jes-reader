//JRS0C4   JOB (ACCT),'JES-READER S0C4',CLASS=A,MSGCLASS=H,
//             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//*--------------------------------------------------------------------
//* CASE S0C4: write to address 0 through a NULL pointer
//* (protection exception). Depending on LE options: S0C4 in step GO,
//* or CEE3204S then U4038.
//*--------------------------------------------------------------------
//COBRUN   EXEC IGYWCLG
//COBOL.SYSIN DD *
       IDENTIFICATION DIVISION.
       PROGRAM-ID. ABEND0C4.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       01  WS-PTR              USAGE POINTER.
       LINKAGE SECTION.
       01  LK-AREA             PIC X(100).
       PROCEDURE DIVISION.
           SET WS-PTR TO NULL
           SET ADDRESS OF LK-AREA TO WS-PTR
           MOVE ALL 'X' TO LK-AREA
           GOBACK.
/*
