//JRIGZ    JOB (ACCT),'JES-READER IGZ',CLASS=A,MSGCLASS=H,
//             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//*--------------------------------------------------------------------
//* CAS IGZ : OPEN d'un fichier dont la DD INFILE est absente du step
//* GO, sans clause FILE STATUS. Le runtime COBOL termine le programme.
//* Attendu : IGZ0035S dans SYSOUT/CEEDUMP, puis abend U4038.
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
