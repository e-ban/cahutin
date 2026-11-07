//! Serveur MCP de mise en page : un assistant (LLM) lit et modifie la disposition d'un document Cahutin
//! sans jamais recevoir ni texte, ni photo, ni nom de fichier.

pub mod agent;
pub mod arrange;
pub mod describe;
pub mod edit;
pub mod file;
pub mod pictos;
pub mod server;
pub mod styles;
pub mod tables;
pub mod tools;
pub mod view;
