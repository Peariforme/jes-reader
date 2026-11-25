# Extension VS Code

Client mince (TypeScript + `vscode-languageclient`) qui lance le serveur `jes-reader-lsp`.

Identifiant : `peariforme.jes-reader`.

## Ce que fait l'extension

- **Language id `jes-sysout`** : extensions `.sysout` et `.joblog`, DD téléchargées par Zowe Explorer (`JOBnnnnn/…/*.txt`), documents ouverts par Zowe (`<job>.<jobid>.<step>.<DD>.<n>`), et détection par la première ligne (bannière JES2, `STMT NO. MESSAGE`, carte JOB numérotée).
- **Coexistence avec IBM Z Open Editor** : le client s'abonne aussi à ses language ids `spool`, `jesmsglg` et `jesjcl`. Z Open Editor garde sa coloration, JES Reader ajoute diagnostics, Outline et repliage.
- **Grammaire TextMate** : identifiants de message colorés selon leur suffixe, avec les messages d'erreur connus qui finissent par `I` (`IEF453I`, `IEF472I`…) en rouge, abends, codes retour au-delà de 4, noms de datasets, instructions JCL.
- **Commandes** : « JES Reader : Ouvrir comme sysout » force le langage du document actif ; « JES Reader : Redémarrer le serveur ».
- **Réglages** : `jesReader.maxReturnCode` (4 par défaut), `jesReader.serverPath` (binaire du serveur, sinon celui de `server/`).

## Développement

```sh
cd editors/vscode
npm ci
npm run build:server   # cargo build --release, copie dans server/
npm run compile
```

Puis ouvrir `editors/vscode` dans VS Code et lancer « Extension JES Reader » (F5) : une seconde fenêtre s'ouvre sur `corpus/sysout`.
