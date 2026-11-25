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

## Packaging et publication

`npm run package -- <cible>` produit `dist/jes-reader-<cible>-<version>.vsix` avec le serveur compilé pour cette plateforme. Cibles : `win32-x64`, `win32-arm64`, `linux-x64`, `linux-arm64`, `darwin-x64`, `darwin-arm64`. Sans argument : la machine courante. Fermer d'abord les fenêtres de développement (F5), qui verrouillent `server/`.

Le workflow `Package` construit les VSIX et les binaires CLI de cinq plateformes à chaque PR et push sur `main` (artefacts de l'exécution). Pour publier une version :

1. Aligner la version dans `editors/vscode/package.json` et `Cargo.toml` (le workflow refuse un tag qui ne correspond pas).
2. Pousser un tag `vX.Y.Z`.
3. Le workflow crée la release GitHub (VSIX et CLI attachés), puis publie sur le VS Code Marketplace si le secret `VSCE_PAT` existe, et sur Open VSX si le secret `OVSX_PAT` existe.

Les deux jetons se créent une fois : `VSCE_PAT` est un Personal Access Token Azure DevOps (portée *Marketplace > Manage*) lié à l'éditeur `peariforme` ; `OVSX_PAT` est un jeton open-vsx.org, après création de l'espace de noms `peariforme` (`npx ovsx create-namespace peariforme`).
