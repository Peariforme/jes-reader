# jes-reader

Serveur de langage qui transforme une sysout z/OS (JES2) en document navigable : erreurs dans le panneau Problems, steps dans l'Outline, messages colorés par sévérité. Tout tourne sur le poste, sans rien installer ni ouvrir côté z/OS.

> Projet en phase 0 (fondations et corpus). Rien n'est encore utilisable. Voir [docs/ROADMAP.md](docs/ROADMAP.md).

## Structure

| Dossier | Rôle |
| --- | --- |
| `crates/jes-reader-core` | Parsing, modèle Job → DD → step → message, moteur de règles |
| `crates/jes-reader-lsp` | Serveur LSP (`lsp-server`, synchrone) |
| `crates/jes-reader-cli` | CLI `jes-reader` (`jes-reader check job.txt`) |
| `editors/vscode` | Extension VS Code (phase 1) |
| `corpus/` | JCL de test et sysouts synthétiques, aucune donnée client |
| `docs/` | Roadmap et notes sur le format des sysouts |

## Développement

Une fois après le clone, activer le filtre qui anonymise le corpus au `git add` (userids Z Xplore remplacés par `Z99999`) :

```sh
git config filter.anonymize.clean "sh scripts/anonymize.sh"
git config filter.anonymize.smudge cat
git config filter.anonymize.required true
```

La copie de travail garde les vraies valeurs ; seul ce qui est commité est anonymisé. La CI échoue si un userid réel passe quand même.

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p jes-reader-cli -- check corpus/sysout/<fichier>
```

## Licence

Au choix, sous [MIT](LICENSE-MIT) ou [Apache-2.0](LICENSE-APACHE).
