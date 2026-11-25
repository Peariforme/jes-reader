# Format des sysouts JES2

Notes d'observation, à compléter à partir du corpus. Chaque affirmation cite le fichier du corpus qui la montre. Les colonnes sont numérotées à partir de 1, colonne ASA comprise.

Sources observées à ce jour :

- `corpus/sysout/JOB00823/` : job batch terminé (validation Z Xplore), une DD applicative `SYSTSPRT`
- `corpus/sysout/TSU00812/` : session TSO ouverte par z/OSMF, **toujours active** au moment du téléchargement

## Récupération via Zowe Explorer

- Zowe Explorer lit les spools **DD par DD** : chaque DD est un document à part, il n'y a pas de fichier « spool complète » par défaut.
- Le téléchargement d'un job produit l'arborescence `<jobid>/<step>/<DD>.txt`. Les DD système sont rangées sous `JES2/` (`JOB00823/JES2/JESMSGLG.txt`), les DD applicatives sous le nom du step (`JOB00823/VALIDATE/SYSTSPRT.txt`).
- Step issu d'une procédure : **le step de la procédure vient en premier**, puis le step du job : `<jobid>/<procstep>/<step>/<DD>.txt` (`JOB00896/GO/COBRUN/SYSOUT.txt`, où `GO` est le step de `IGYWCLG` et `COBRUN` le step du job).
- Zowe ajoute parfois un suffixe `(1)` au nom de fichier, sans doublon apparent dans le même dossier (`JOB00896/LKED/COBRUN/SYSPRINT(1).txt`). Le nom de DD s'obtient en retirant ce suffixe.
- Des produits installés ajoutent leurs propres DD : IBM Fault Analyzer écrit `IDIREPRT` sur les abends (`JOB00901/STEP010/IDIREPRT.txt`, 2 448 lignes).
- Ouvert sans téléchargement, le document porte un nom de la forme `<owner>.<jobid>.<step>.<DD>.<n>` (exemple : `Z99999.TSU00812.JES2.JESYSMSG.4`), où `n` est le numéro de la DD dans la spool.
- Conséquence pour le parser : **l'unité de parsing est la DD**. Le nom du fichier (ou l'URI) donne la DD ; le contenu sert de repli (voir « Détection »).
- Spool complète en un seul fichier : _à observer_ (séparateur entre DD inconnu à ce jour).

## Caractère ASA (colonne 1)

- **Présent dans toutes les DD, même téléchargées une par une.**
- `1` saut de page : 1re ligne de JESMSGLG (`JOB00823/JES2/JESMSGLG.txt`) et de `SYSTSPRT` (`1READY`, `JOB00823/VALIDATE/SYSTSPRT.txt`)
- `0` double interligne : 2e ligne de JESMSGLG, et `0------ JES2 JOB STATISTICS ------`
- `-` triple interligne : lignes des statistiques JES2 en fin de JESMSGLG
- ` ` simple interligne : tout le reste ; JESJCL et JESYSMSG n'ont que des espaces en colonne 1
- `+` surimpression : _pas encore observé_
- Impact sur le parser : retirer la colonne 1 avant toute analyse, mais la garder dans les positions renvoyées au LSP (les colonnes de l'éditeur incluent l'ASA).

## Largeur des lignes et fins de ligne

- Longueur maximale observée : **133** (132 + ASA), sur le tableau des steps de JESMSGLG (`JOB00823/JES2/JESMSGLG.txt`).
- **Espaces de fin tronqués** dans tous les fichiers : la longueur des lignes varie, aucune ligne ne finit par un espace.
- Fins de ligne **CRLF** dans les fichiers téléchargés sous Windows, y compris la dernière ligne. Origine (z/OSMF ou Zowe) _à vérifier_ ; le parser doit accepter LF et CRLF.
- Le corpus est versionné sans conversion (`.gitattributes`).

## JESMSGLG

```
1                   J E S 2  J O B  L O G  --  S Y S T E M  S 0 W 1  --  N O D E  S V S C J E S 2
0
 04.12.28 JOB00823 ---- WEDNESDAY, 25 NOV 2026 ----
 04.12.28 JOB00823  IRR010I  USERID Z99999   IS ASSIGNED TO THIS JOB.
```

- Ligne 1 : bannière, système (`S0W1`) et nœud JES2 (`SVSCJES2`).
- Ligne de date : `---- <JOUR>, <jj> <MMM> <aaaa> ----`.
- Messages : heure `hh.mm.ss` (col. 2-9), identifiant JES (col. 11-18), deux espaces, identifiant du message à partir de la col. 21.
- **Identifiant JES** : préfixe `JOB` pour un batch, `TSU` pour une session TSO (`TSU00812/JES2/JESMSGLG.txt`), `STC` pour une tâche démarrée (_à observer_). Le parser accepte les trois.
- Tableau récapitulatif des steps, préfixé par `-` après l'identifiant JES (`JOB00823/JES2/JESMSGLG.txt`) :

  ```
   04.12.31 JOB00823  -STEPNAME PROCSTEP    RC   EXCP   CONN       TCB       SRB  CLOCK          SERV  WORKLOAD  PAGE  SWAP   VIO SWAPS
   04.12.31 JOB00823  -VALIDATE             00    205      8       .00       .00     .0            23  BATCH        0     0     0     0
   04.12.31 JOB00823  -@VSC1    ENDED.  NAME-                     TOTAL TCB CPU TIME=      .00 TOTAL ELAPSED TIME=    .0
  ```

  Il donne le RC de chaque step sans avoir besoin de JESYSMSG. Un step non exécuté affiche `FLUSH` dans la colonne `RC`, décalé sous `PROCSTEP` (`JOB00889/JES2/JESMSGLG.txt` : `-STEP010           FLUSH`). Format exact selon l'installation (exit IEFACTRT) : _à confirmer sur d'autres systèmes_.
- Fin de job, suivie du bloc `JES2 JOB STATISTICS`. Trois formes de `$HASP395` :
  - `ENDED - RC=0008` : RC max des steps (`JOB00868`)
  - `ENDED - ABEND=S0C4` ou `ABEND=U4038` (`JOB00893`, `JOB00885`)
  - `ENDED` seul, après `IEF453I <job> - JOB FAILED - JCL ERROR` (`JOB00889`, `JOB00890`)
- Abend : `IEF450I <job> <procstep> <step> - ABEND=S000 U4038 REASON=00000001` dans JESMSGLG (`JOB00885`).
- **Job en cours** : JESMSGLG s'arrête après `$HASP373 … STARTED`, sans `$HASP395` ni statistiques (`TSU00812/JES2/JESMSGLG.txt`). L'absence de fin n'est pas une erreur.

## JESJCL

```
        1 //@VSC1     JOB ,MSGLEVEL=(0,0),CLASS=7                                 JOB00823
```

- Numéro d'instruction cadré à droite en col. 2-9, puis un espace, puis l'instruction à partir de la col. 11. Attention : dans JESYSMSG, le numéro est décalé d'une colonne (col. 2-10).
- Lignes de continuation : numéro à blanc (`TSU00812/JES2/JESJCL.txt`, `//             REGION=65536K`).
- Instructions issues d'une procédure : `XX` au lieu de `//` (`XXIZUFPROC PROC …`).
- Substitutions de symboles : `IEFC653I SUBSTITUTION JCL - …` à partir de la col. 11, sans numéro.
- La carte JOB porte l'identifiant JES en fin de ligne (col. 83-90).
- Ces numéros d'instruction sont la cible des liens depuis la colonne `STMT NO.` de JESYSMSG.

## JESYSMSG

```
  STMT NO. MESSAGE
         2 IEFC001I PROCEDURE ZXP$$CHK WAS EXPANDED USING SYSTEM LIBRARY VENDOR.PROCLIB
 IEF142I @VSC1 VALIDATE - STEP WAS EXECUTED - COND CODE 0000
```

- 1re ligne : en-tête `STMT NO. MESSAGE`.
- Messages de conversion JCL : numéro d'instruction en col. 2-10, identifiant à partir de la col. 12.
- Messages d'exécution : identifiant en col. 2.
- Lignes de suite d'un message : en général indentées (col. 10 ou plus), sans identifiant (`CPU: …`, `VIRT: …`, `SWA=ABOVE,…`).
- **Piège** : certaines suites commencent en col. 2, comme un nouveau message. Exemples : `IGD17101I` puis ` NOT DEFINED BECAUSE DUPLICATE NAME EXISTS IN CATALOG` (`JOB00890`), `IEC141I 013-18,…` puis ` SYS1.MACLIB(JRNOMBR)` (`JOB00901`). Une ligne en col. 2 n'ouvre un nouveau message que si elle commence par un identifiant valide ; sinon, elle prolonge le message précédent.
- Nom de step dans les messages : `<job> <step>` pour un step direct, `<job> <procstep> <step>` pour un step de procédure (`IEF142I JRS0C7 COBOL COBRUN - …`).
- Horodatage des steps : `IEF373I STEP/<step>/START aaaajjj.hhmm` et `IEF032I STEP/<step>/STOP aaaajjj.hhmm` (date julienne).
- Fin de job : `IEF375I JOB/<job>/START …` et `IEF033I JOB/<job>/STOP …`.

## Détection (contenu, en repli du nom de fichier)

| DD | Motif fiable |
| --- | --- |
| JESMSGLG | ligne 1 : `J E S 2  J O B  L O G` |
| JESJCL | ligne 1 : numéro `1` en col. 9, `//` en col. 11, carte `JOB` |
| JESYSMSG | ligne 1 : `STMT NO. MESSAGE` |
| Autres DD | pas de motif général : nom de fichier ou URI obligatoire |

## Messages clés

| Message | DD | Rôle | Exemple |
| --- | --- | --- | --- |
| `$HASP373` | JESMSGLG | Début du job | `$HASP373 @VSC1    STARTED - INIT 1    - CLASS 7        - SYS S0W1` |
| `$HASP395` | JESMSGLG | Fin du job avec code retour max | `$HASP395 @VSC1    ENDED - RC=0000` |
| `IEF142I` | JESYSMSG | Fin de step avec code retour | `IEF142I @VSC1 VALIDATE - STEP WAS EXECUTED - COND CODE 0000` |
| `IEF373I` / `IEF032I` | JESYSMSG | Début / fin de step | `IEF373I STEP/VALIDATE/START 2026329.0412` |
| `IEF375I` / `IEF033I` | JESYSMSG | Début / fin de job | `IEF375I  JOB/@VSC1   /START 2026329.0412` |
| `IEFC001I` | JESYSMSG | Procédure développée | `2 IEFC001I PROCEDURE ZXP$$CHK WAS EXPANDED …` |
| `IEF472I` | JESYSMSG | Fin anormale (abend) | `IEF472I JRS0C7 GO COBRUN - COMPLETION CODE - SYSTEM=0C7 USER=0000 REASON=00000007` |
| `IEF450I` | JESMSGLG | Fin anormale (abend) | `IEF450I JRIGZ GO COBRUN - ABEND=S000 U4038 REASON=00000001` |
| `IEFA107I` | JESYSMSG | Dataset introuvable à l'allocation (pas `IEF212I` sur Z Xplore) | `IEFA107I JRJCLERR STEP010 SYSUT1 - DATA SET Z99999.JESREAD.INEXIST NOT FOUND` |
| `IEF202I` | JESYSMSG | Step non exécuté (COND) | `IEF202I JRIDC8 STEP020 - STEP WAS NOT RUN BECAUSE OF CONDITION CODES` |
| `IEF272I` | JESYSMSG | Step non exécuté (suit `IEF202I` ou une erreur d'allocation) | `IEF272I JRIDC8 STEP020 - STEP WAS NOT EXECUTED.` |
| `IEF453I` | JESMSGLG | JCL ERROR | `IEF453I JRJCLERR - JOB FAILED - JCL ERROR` |
| `IGD17101I` | JESYSMSG | Dataset en double (SMS), suite en col. 2 | `IGD17101I DATA SET Z99999.JESREAD.DUP` |
| `IEC141I` | JESYSMSG | Abend d'OPEN (013) | `IEC141I 013-18,IGG0191B,JRS013,STEP010,SYSUT1,1001,VTMVSC,` |
| `CSV003I` | JESYSMSG | Module introuvable (S806) | `CSV003I REQUESTED MODULE JRNOPGM  NOT FOUND` |
| `IGZ0035S` | SYSOUT (step GO) | OPEN COBOL en échec, suivi de U4038 | `IGZ0035S There was an unsuccessful OPEN or CLOSE of file INFILE …` |
