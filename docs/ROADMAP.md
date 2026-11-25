# Roadmap — Serveur de langage Sysout z/OS

Nov 25, 2026 · @Richard

## Vision et périmètre

Un serveur de langage qui transforme une sysout z/OS en document navigable : erreurs dans le panneau Problems, steps dans l'Outline, messages colorés par sévérité. Il tourne entièrement sur le poste, sans rien installer ni ouvrir côté z/OS.

**Pour qui** : développeurs et exploitants mainframe sur poste verrouillé, où VS Code est autorisé mais SSH et USS ne le sont pas.

**Dans le périmètre (v1)**

- Sysouts JES2 téléchargées en fichier, ou ouvertes directement depuis Zowe Explorer
- DD système (JESMSGLG, JESJCL, JESYSMSG) et applicatives (SYSPRINT, SYSOUT, CEEDUMP lu comme texte)
- Messages IBM courants (IEF, IEC, IGD, $HASP, CEE, IGZ, DSN, DFS) et règles maison déclarées par l'utilisateur

**Hors périmètre (v1)**

- Toute connexion au mainframe : Zowe Explorer s'en charge déjà
- JES3, à réévaluer selon le corpus
- Interprétation des dumps
- Reproduction des manuels IBM (copyright) : résumés maison et liens vers IBM Docs

## Vue d'ensemble

&#91;embedded content: Roadmap · 5 phases, 5 critères de sortie\]

Chaque phase se termine par un critère de sortie vérifiable. L'outil devient utile au quotidien dès la fin de la phase 1.

## Choix techniques

Un seul moteur Rust, exposé par un serveur LSP pour les éditeurs et par une CLI pour la CI.

&#91;embedded content: Architecture · 1 moteur, 2 façades, 3 usages\]

| Brique | Choix | Pourquoi |
| --- | --- | --- |
| Langage | Rust | Binaire unique, pas de Node à installer sur le poste, parsing rapide |
| Protocole | LSP (tower-lsp-server ou lsp-server, à trancher) | Consommé par VS Code, Zed, Neovim et Eclipse |
| Coloration | Grammaire TextMate + semantic tokens | TextMate marche même sans serveur ; les semantic tokens portent la sévérité |
| Extension VS Code | TypeScript + vscode-languageclient | Client mince, binaire embarqué par plateforme |
| CLI | clap | `sysout check job.txt`, sortie texte, JSON ou SARIF |
| Tests | insta (snapshots) | Chaque sysout du corpus a son analyse attendue versionnée |

**Structure du dépôt** (workspace Cargo)

- `crates/sysout-core` : parsing, modèle Job → DD → step → message, moteur de règles
- `crates/sysout-lsp` : serveur LSP
- `crates/sysout-cli` : CLI
- `editors/vscode` : extension VS Code
- `editors/zed` : extension Zed (phase 4)
- `corpus/` : sysouts de test et snapshots attendus

## Phase 0 — Fondations et corpus (≈ 1 semaine)

Objectif : un dépôt qui compile et un corpus de sysouts réelles, sans aucune donnée client.

- [x] Ouvrir l'accès IBM Z Xplore et vérifier qu'on peut compiler du COBOL et soumettre des jobs (soumission validée par `JOB00823`, compilation COBOL à confirmer avec `s0c7.jcl`)
- [x] Écrire des jobs qui plantent volontairement : S0C7, S0C4, S806, S322, S013, JCL ERROR, dataset introuvable, NOT CATLGD, RC 4/8/12 via IDCAMS, abend utilisateur COBOL (IGZ) (RC 12 IDCAMS restant)
- [ ] Récupérer chaque sysout via Zowe Explorer. Constat : Zowe lit et télécharge les spools DD par DD (`<jobid>/<step>/<DD>.txt`), il n'y a pas de spool complète par défaut. Le parser travaille donc DD par DD (voir `FORMAT.md`)
- [x] Créer le workspace Cargo et la CI GitHub Actions (fmt, clippy, tests)
- [x] Choisir la librairie LSP : `lsp-server`
- [ ] Tenir un `FORMAT.md` (commencé à partir de `JOB00823` et `TSU00812`) : caractère ASA en colonne 1, largeur des lignes, séparateurs de DD, en-têtes de page, marqueur d'en-tête JES2

**Sortie** : au moins 15 sysouts versionnées et une CI verte.

## Phase 1 — MVP (≈ 4 semaines)

Objectif : ouvrir une sysout dans VS Code et voir tout de suite quel step a planté et pourquoi.

1. **Parser** (`jes-reader-core`) : **l'unité de parsing est la DD** (`parse_dd`), car Zowe Explorer ouvre les spools DD par DD ; une spool complète est d'abord découpée en DD. Type de DD tiré du nom de fichier ou de l'URI, à défaut du contenu. Le `Job` est un agrégat facultatif des DD ouvertes du même identifiant JES (`JOB`, `TSU` ou `STC`). Puis, détection des steps et de leur fin (messages IEF de début, fin et code retour), extraction des messages par ID avec sévérité tirée du dernier caractère (I, W, E, S, A). Le parser ne plante jamais : ce qu'il ne comprend pas reste du texte brut.
2. **Détection du langage** : language id `jes-sysout` par extension (`.sysout`, `.joblog`), par nom de DD et par contenu (en-tête JES2 dans les premières lignes), plus une commande « Ouvrir comme sysout ». Le client s'abonne aussi aux language ids d'IBM Z Open Editor (`spool`, `jesmsglg`, `jesjcl`, attribués par nom de fichier) : quand cette extension est installée, elle garde sa coloration TextMate de base et nous ajoutons diagnostics, outline et semantic tokens par-dessus, sans conflit. Z Open Editor n'a pas de serveur de langage pour les spools (vérifié sur la 6.9.0). Ne rien reprendre de sa grammaire (propriété IBM).
3. **Grammaire TextMate** : IDs de message, codes retour, codes d'abend, noms de datasets et de DD.
4. **Serveur LSP** : outline DD → steps, repliage, diagnostics (abend et JCL ERROR en erreur, RC > 4 en avertissement, seuil configurable).
5. **Extension VS Code** : client mince, binaire embarqué.
6. **Tests** : un snapshot par sysout du corpus.

**Sortie** : sur tout le corpus, le step fautif apparaît en tête du panneau Problems et l'outline est juste.

**État (25/11/2026)** : terminée. Les six points sont implémentés et le critère de sortie est vérifié à la main dans VS Code (Outline, Problems, détection du langage). Les semantic tokens (sévérité portée par le serveur) ne sont pas faits : la grammaire TextMate colore déjà les messages d'erreur connus. Constats :

- La gravité ne se lit pas sur le suffixe : `IEF453I`, `IEF472I`, `IEFA107I` finissent par `I`. Le moteur de règles (`analysis.rs`) et la grammaire partagent une liste de messages connus, à garder synchronisées.
- La sévérité vient des steps (abend, RC, non exécuté), puis de cette liste, puis du suffixe `E`/`S`/`W`.
- `IDI0078E` (Fault Analyzer sans accès à son historique) sort en erreur alors qu'il ne concerne pas le job : cas typique pour les règles maison de la phase 3.

## Phase 2 — Zowe Explorer et hovers (≈ 3 semaines)

Objectif : l'outil s'utilise sur les jobs du quotidien, sans télécharger de fichier.

- [x] Identifier le schéma d'URI des spools ouvertes par Zowe Explorer (`zowe-jobs:`) et y abonner l'extension. Vérifié le 25/11/2026 : un job ouvert directement depuis Zowe Explorer est analysé (diagnostics, Outline). Reste à faire : pour les liens entre DD, l'extension lit à la demande la seule DD manquante via le système de fichiers de Zowe, sans télécharger toute la spool
- [ ] Hovers : catalogue maison en TOML des \~50 messages et abends les plus fréquents, rédigé par nous, avec lien IBM Docs pour tous les autres
- [ ] Liens : numéros d'instruction de JESYSMSG vers la ligne correspondante de JESJCL
- [ ] Navigation : d'un step dans JESYSMSG vers son EXEC dans JESJCL
- [ ] Noms de datasets cliquables vers Zowe Explorer, si une commande publique le permet

**Sortie** : un job ouvert depuis Zowe Explorer est analysé sans aucun téléchargement.

## Phase 3 — Résumés, règles maison, CLI (≈ 3 semaines)

Objectif : l'outil s'adapte aux conventions de chaque client et sert aussi hors éditeur.

- [ ] Code lens par step : nom, programme, code retour, CPU et temps écoulé quand les messages de fin de step les donnent
- [ ] Règles maison dans un `.sysoutrc.toml` (projet ou utilisateur) : motif → sévérité, message, texte de survol. Cas typiques : messages applicatifs, RC tolérés par programme (IDCAMS RC=4 accepté, par exemple)
- [ ] CLI : `sysout check` (code de sortie non nul en cas d'erreur), `sysout summary`, sorties texte, JSON et SARIF

**Sortie** : un fichier de règles d'une dizaine de lignes suffit à adapter l'outil à une application client.

## Phase 4 — Distribution (≈ 2 semaines)

Objectif : installable en un clic chez soi, et sans réseau sur un poste bancaire.

- [ ] Releases cross-compilées via GitHub Actions (Windows x64, Linux x64, macOS arm64)
- [ ] Publication sur le VS Code Marketplace et Open VSX
- [ ] VSIX par plateforme, installé hors-ligne et testé sur un poste Windows sans droits admin
- [ ] Extension Zed : déclaration du langage et lancement du LSP. Zed n'utilise pas TextMate, il faut donc écrire une grammaire tree-sitter pour la coloration
- [ ] README avec capture animée, licence, annonce sur le Slack Open Mainframe Project

**Sortie** : un VSIX hors-ligne fonctionne sur un poste Windows sans droits admin.

## Risques et questions ouvertes

Le risque principal n'est pas technique : c'est la propriété intellectuelle vis-à-vis de l'employeur, à vérifier avant toute publication.

| Risque | Conséquence | Parade |
| --- | --- | --- |
| Clause de propriété intellectuelle du contrat Sopra Steria | Publication bloquée ou contestée | Relire la clause, développer uniquement sur temps et matériel perso |
| Format variable selon l'outil d'export | Parsing fragile | Corpus multi-sources, parser tolérant qui dégrade au lieu de planter |
| Corpus limité à Z Xplore | Messages de prod absents | Règles maison, collecte de motifs (jamais de données) auprès des utilisateurs |
| Confidentialité des sysouts clients | Impossible de partager un cas réel | Aucune sysout client dans le dépôt, cas reproduits en synthétique |
| Copyright de la doc IBM | Hovers pauvres | Résumés rédigés par nous, liens vers IBM Docs |
| Changement du schéma d'URI de Zowe Explorer | Intégration cassée | Test d'intégration sur une version épinglée |

**Questions ouvertes**

- ~~Nom du projet~~ : `jes-reader`
- ~~Licence~~ : MIT ou Apache-2.0 au choix
- JES3 dans la v1 ou plus tard
- ~~tower-lsp-server ou lsp-server~~ : `lsp-server`

## Demain : session 1

Première session de 2 à 3 heures, consacrée à lancer la phase 0.

- [x] S'inscrire à IBM Z Xplore (l'activation peut prendre du temps, à lancer en premier)
- [x] Trancher le nom de code et la librairie LSP
- [x] Initialiser le workspace Cargo (`core`, `lsp`, `cli`) et la CI
- [x] Écrire les trois premiers jobs qui plantent : S0C7, JCL ERROR, IDCAMS RC=8
- [ ] Récupérer leurs sysouts (`FORMAT.md` commencé)
