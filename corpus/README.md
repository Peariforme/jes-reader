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

| Dossier | Cas | Intérêt |
| --- | --- | --- |
| `sysout/JOB00823` | Job de validation Z Xplore, RC=0 | Job batch complet, une DD applicative (`SYSTSPRT`) |
| `sysout/TSU00812` | Session TSO ouverte par z/OSMF | Préfixe `TSU`, procédure `IZUFPROC`, **session active** : pas de fin de job |

## Cas à soumettre

| Cas | JCL | Attendu | Sysout |
| --- | --- | --- | --- |
| S0C7 | `jcl/s0c7.jcl` | Abend S0C7 (ou CEE3207S / U4038 selon les options LE) au step GO | à récupérer |
| S0C4 | `jcl/s0c4.jcl` | Abend S0C4 (ou CEE3204S / U4038) au step GO | à récupérer |
| S806 | `jcl/s806.jcl` | `CSV003I`, abend S806-04 | à récupérer |
| S322 | `jcl/s322.jcl` | Boucle infinie, abend S322 au step GO (`TIME=(0,1)`) | à récupérer |
| S013 | `jcl/s013.jcl` | `IEC141I 013-18`, membre absent | à récupérer |
| NOT CATLGD | `jcl/not-catlgd.jcl` | `IGD17101I` (SMS) ou `IEF287I … NOT CATLGD 2` (non SMS) au STEP020 | à récupérer |
| IGZ | `jcl/igz-open.jcl` | `IGZ0035S` (OPEN sans DD), puis U4038 | à récupérer |
| JCL ERROR | `jcl/jcl-error.jcl` | `IEF212I ... DATA SET NOT FOUND`, job non exécuté | à récupérer |
| IDCAMS RC=8 | `jcl/idcams-rc8.jcl` | RC 4 (LISTCAT), RC 8 (DELETE), step suivant non exécuté | à récupérer |

Sur Z Xplore, remplacer `CLASS=A` par la classe autorisée (`CLASS=7` observée sur `JOB00823`) et vérifier le nom de la procédure de compilation COBOL (`IGYWCLG` par défaut). Pour chaque job, télécharger la spool avec l'action de téléchargement de Zowe Explorer (clic droit sur le job), puis la déposer telle quelle dans `sysout/`.
