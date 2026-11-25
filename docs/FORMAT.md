# Format des sysouts JES2

Notes d'observation, à compléter à partir du corpus. Chaque affirmation cite le fichier du corpus qui la montre. Les colonnes sont numérotées à partir de 1, colonne ASA comprise.

Sources observées à ce jour :

- `corpus/sysout/JOB00823/` : job batch terminé (validation Z Xplore), une DD applicative `SYSTSPRT`
- `corpus/sysout/TSU00812/` : session TSO ouverte par z/OSMF, **toujours active** au moment du téléchargement

## Récupération via Zowe Explorer

- Zowe Explorer lit les spools **DD par DD** : chaque DD est un document à part, il n'y a pas de fichier « spool complète » par défaut.
- Le téléchargement d'un job produit l'arborescence `<jobid>/<step>/<DD>.txt`. Les DD système sont rangées sous `JES2/` (`JOB00823/JES2/JESMSGLG.txt`), les DD applicatives sous le nom du step (`JOB00823/VALIDATE/SYSTSPRT.txt`).
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

  Il donne le RC de chaque step sans avoir besoin de JESYSMSG. Format exact selon l'installation (exit IEFACTRT) : _à confirmer sur d'autres systèmes_.
- Fin de job : `$HASP395 <job> ENDED - RC=0000`, suivi du bloc `JES2 JOB STATISTICS`.
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
- Lignes de suite d'un message : indentées (col. 10 ou plus), sans identifiant (`CPU: …`, `VIRT: …`, `SWA=ABOVE,…`).
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
| `IEF472I` | JESYSMSG | Fin anormale (abend) | _à remplir (s0c7, s0c4, s806…)_ |
| `IEF212I` | JESYSMSG | Dataset introuvable | _à remplir (jcl-error)_ |
| `IEF202I` | JESYSMSG | Step non exécuté (COND) | _à remplir (idcams-rc8)_ |
| `IEF453I` | JESMSGLG | JCL ERROR | _à remplir (jcl-error)_ |
