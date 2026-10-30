//! Moteur de l'application : modèle de document, import, formats et export.

pub mod background;
pub mod class;
pub mod crop;
pub mod examples;
pub mod fill;
pub mod fonts;
mod fsutil;
pub mod grid;
pub mod groups;
pub mod import;
pub mod model;
pub mod native;
pub mod odg;
pub mod photo;
pub mod picto;
pub mod report;
pub mod shapes;
pub mod table;
pub mod templates;

pub use crop::{SourceRect, source_rect};
pub use model::*;
