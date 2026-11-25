# Corpus

Sysouts de test et JCL qui les produit.

**Règle absolue : aucune sysout client, aucune donnée réelle.** Tout ce qui se trouve ici est produit sur IBM Z Xplore à partir des jobs de `jcl/`, ou reconstitué à la main de façon synthétique.

Les userids Z Xplore (`Z` suivi de 5 chiffres) sont remplacés par `Z99999` au `git add` par le filtre `anonymize` (voir le README à la racine pour l'activer). La CI échoue si un userid réel passe quand même.

## Organisation

- `jcl/` : jobs qui plantent volontairement, un cas par fichier
- `sysout/` : sysouts récupérées, **dans l'arborescence produite par Zowe Explorer**, sans renommage. C'est ce que le parser recevra en vrai :

  ```
  sysout/<jobid>/JES2/JESMSGLG.txt     DD système
  sysout/<jobid>/JES2/JESJCL.txt
  sysout/<jobid>/JES2/JESYSMSG.txt
  sysout/<jobid>/<step>/<DD>.txt        DD applicatives
  ```

  Une spool complète téléchargée en un seul fichier, si un outil le permet, va dans `sysout/<jobid>.spool.txt`.

Les fichiers de ce dossier ne subissent aucune conversion de fin de ligne (voir `.gitattributes`). Ne pas les reformater.

## Sysouts récupérées

| Dossier | Cas | JCL | Résultat | Intérêt |
| --- | --- | --- | --- | --- |
| `sysout/JOB00823` | Validation Z Xplore | (fourni par Z Xplore) | RC=0 | DD applicative `SYSTSPRT` |
| `sysout/TSU00812` | Session TSO (z/OSMF) | | session active | Préfixe `TSU`, pas de fin de job |
| `sysout/JOB00868` | IDCAMS RC=8 | `jcl/idcams-rc8.jcl` | RC 4, RC 8, STEP020 `IEF202I` | `$HASP395 ENDED - RC=0008` |
| `sysout/JOB00869` | JCL ERROR dans une procédure | `jcl/igz-open.jcl` sans `<userid>.LOAD` | step COBOL RC=0, LKED `IEFA107I` sur `SYSLMOD`, `IEF453I` | JCL ERROR après un step exécuté ; avertissements de conversion numérotés (`IEF655I`, `IEF686I`) |
| `sysout/JOB00884` | Allocation de LOAD | `jcl/setup-load.jcl` | RC=0 | Job minimal IEFBR14 |
| `sysout/JOB00885` | IGZ | `jcl/igz-open.jcl` | `IGZ0035S`, U4038 | Steps de procédure, DD `SYSOUT` du step GO |
| `sysout/JOB00889` | JCL ERROR | `jcl/jcl-error.jcl` | `IEFA107I`, `IEF453I` | `FLUSH` dans le tableau des steps |
| `sysout/JOB00890` | NOT CATLGD | `jcl/not-catlgd.jcl` | `IGD17101I` (SMS), JCL ERROR | Message dont la suite est en col. 2 |
| `sysout/JOB00893` | S0C4 | `jcl/s0c4.jcl` | S0C4 au step GO | |
| `sysout/JOB00896` | S0C7 | `jcl/s0c7.jcl` | S0C7 au step GO | |
| `sysout/JOB00901` | S013 | `jcl/s013.jcl` | `IEC141I 013-18` | Rapport Fault Analyzer (`IDIREPRT`) |
| `sysout/JOB00903` | S322 | `jcl/s322.jcl` | S322 au step GO | |
| `sysout/JOB00905` | S806 | `jcl/s806.jcl` | `CSV003I`, S806-04 | Rapport Fault Analyzer (`IDIREPRT`) |

Sur Z Xplore, S0C7 et S0C4 sortent en abend système, sans conversion LE en U4038.

## Cas restants

| Cas | JCL et attendu |
| --- | --- |
| IDCAMS RC=12 | `jcl/idcams-rc12.jcl` : mot-clé inconnu, `$HASP395 … RC=0012` |
| Job long | `jcl/long.jcl` : une dizaine de steps, `IF/THEN/ELSE`, procédure in-stream appelée deux fois, SYSPRINT de plusieurs dizaines de pages, DD écrite sans ASA (IEBGENER) |

Avant les jobs COBOL (s0c7, s0c4, s322, igz-open), soumettre une fois `jcl/setup-load.jcl` : la procédure `IGYWCLG` de Z Xplore range le module dans `<userid>.LOAD` et le job part en JCL ERROR si la bibliothèque n'existe pas.

Sur Z Xplore, remplacer `CLASS=A` par la classe autorisée (`CLASS=7` observée sur `JOB00823`) et vérifier le nom de la procédure de compilation COBOL (`IGYWCLG` par défaut). Pour chaque job, télécharger la spool avec l'action de téléchargement de Zowe Explorer (clic droit sur le job), puis la déposer telle quelle dans `sysout/`.
