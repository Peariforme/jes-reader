//! Moteur d'analyse des sysouts JES2 z/OS.
//!
//! Le modèle suit la hiérarchie Job → DD → step → message. Le parser,
//! qui arrive en phase 1, ne plante jamais : ce qu'il ne comprend pas
//! reste du texte brut.

pub mod model;

pub use model::{Dd, Job, Message, Severity, Step};
