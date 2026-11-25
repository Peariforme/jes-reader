# Corpus

Sysouts de test et JCL qui les produit.

**Règle absolue : aucune sysout client, aucune donnée réelle.** Tout ce qui se trouve ici est produit sur IBM Z Xplore à partir des jobs de `jcl/`, ou reconstitué à la main de façon synthétique.

## Organisation

- `jcl/` : jobs qui plantent volontairement, un cas par fichier
- `sysout/` : sysouts récupérées. Nommage : `<cas>.<source>.sysout`, par exemple
  - `s0c7.spool.sysout` : spool complète téléchargée via Zowe Explorer
  - `s0c7.JESYSMSG.sysout` : une seule DD

Les fichiers de ce dossier ne subissent aucune conversion de fin de ligne (voir `.gitattributes`). Ne pas les reformater.

## Cas prévus (phase 0)

| Cas | JCL | Attendu |
| --- | --- | --- |
| S0C7 | `jcl/s0c7.jcl` | Abend S0C7 (ou CEE3207S / U4038 selon les options LE) au step GO |
| JCL ERROR | `jcl/jcl-error.jcl` | `IEF212I ... DATA SET NOT FOUND`, job non exécuté |
| IDCAMS RC=8 | `jcl/idcams-rc8.jcl` | RC 4 (LISTCAT), RC 8 (DELETE), step suivant non exécuté |
| S0C4, S806, S322, S013, NOT CATLGD, IGZ | à écrire | |

Sur Z Xplore, remplacer la carte JOB si nécessaire et vérifier le nom de la procédure de compilation COBOL (`IGYWCLG` par défaut).
