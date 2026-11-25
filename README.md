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

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p jes-reader-cli -- check corpus/sysout/<fichier>
```

## Licence

Au choix, sous [MIT](LICENSE-MIT) ou [Apache-2.0](LICENSE-APACHE).
