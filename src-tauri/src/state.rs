//! État partagé : les images, la bibliothèque de modèles et les polices. Le document vit dans l'interface.

use carnet_core::fonts::FontSet;
use carnet_core::photo::AssetStore;
use carnet_core::templates::Library;
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};

pub struct AppState {
    pub store: Mutex<AssetStore>,
    pub library: Library,
    /// Polices fournies et polices de `fonts_dir`, relues après chaque ajout ou suppression.
    pub fonts: RwLock<FontSet>,
    pub fonts_dir: PathBuf,
    /// Polices transportées par le dernier document ouvert et absentes du poste : (fichier, octets).
    pub pending_fonts: Mutex<Vec<(String, Vec<u8>)>>,
}

impl AppState {
    pub fn new(store: AssetStore, library: Library, fonts_dir: PathBuf) -> AppState {
        AppState {
            store: Mutex::new(store),
            library,
            fonts: RwLock::new(FontSet::with_user_dir(&fonts_dir)),
            fonts_dir,
            pending_fonts: Mutex::new(Vec::new()),
        }
    }

    pub fn reload_fonts(&self) {
        *self.fonts.write().unwrap() = FontSet::with_user_dir(&self.fonts_dir);
    }
}
