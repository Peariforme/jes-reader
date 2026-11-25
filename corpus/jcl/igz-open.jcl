//JRIGZ    JOB (ACCT),'JES-READER IGZ',CLASS=A,MSGCLASS=H,
//             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//*--------------------------------------------------------------------
//* CASE IGZ: OPEN of a file whose INFILE DD is missing from step
//* GO, with no FILE STATUS clause. The COBOL runtime ends the program.
//* Expected: IGZ0035S in SYSOUT/CEEDUMP, then abend U4038.
//*--------------------------------------------------------------------
//COBRUN   EXEC IGYWCLG
//COBOL.SYSIN DD *
       IDENTIFICATION DIVISION.
       PROGRAM-ID. ABENDIGZ.
       ENVIRONMENT DIVISION.
       INPUT-OUTPUT SECTION.
       FILE-CONTROL.
           SELECT IN-FILE ASSIGN TO INFILE.
       DATA DIVISION.
       FILE SECTION.
       FD  IN-FILE RECORDING MODE F.
       01  IN-REC              PIC X(80).
       PROCEDURE DIVISION.
           OPEN INPUT IN-FILE
           CLOSE IN-FILE
           GOBACK.
/*
