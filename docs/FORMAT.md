# Format des sysouts JES2

Notes d'observation, à compléter à partir du corpus. Chaque affirmation cite le fichier du corpus qui la montre.

## Caractère ASA (colonne 1)

- Présent ou non selon l'outil d'export : _à vérifier (spool Zowe vs DD par DD)_
- Valeurs : `1` saut de page, ` ` simple interligne, `0` double, `-` triple, `+` surimpression
- Impact sur le parser : _à décrire_

## Largeur des lignes

- LRECL observées : _à remplir_ (133 avec ASA ? 132 sans ?)
- Espaces de fin conservés ou tronqués : _à remplir_

## Séparation des DD

- Spool complète Zowe : _séparateur observé à recopier_
- Ordre habituel : JESMSGLG, JESJCL, JESYSMSG, puis DD applicatives

## En-têtes de page

- Bannières JES2 (`J E S 2  J O B  L O G`) : _à recopier_
- Ligne de date et de système : _à remplir_

## Marqueur d'en-tête JES2 (détection du langage)

- Motif fiable dans les N premières lignes : _à définir_ (`$HASP373`, bannière JES2 ?)

## Messages clés

| Message | DD | Rôle | Exemple |
| --- | --- | --- | --- |
| `IEF142I` | JESYSMSG | Fin de step avec code retour | _à remplir_ |
| `IEF472I` | JESYSMSG | Fin anormale (abend) | _à remplir_ |
| `IEF212I` | JESYSMSG | Dataset introuvable | _à remplir_ |
| `IEF202I` | JESYSMSG | Step non exécuté (COND) | _à remplir_ |
| `IEF404I` / `IEF453I` | JESMSGLG | Fin de job normale / JCL ERROR | _à remplir_ |
