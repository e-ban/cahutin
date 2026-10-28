//! État partagé : les images, la bibliothèque de modèles et les polices. Le document vit dans l'interface.

use carnet_core::fonts::{FontFace, FontSet};
use carnet_core::photo::AssetStore;
use carnet_core::templates::Library;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, RwLock};

pub struct AppState {
    pub store: Mutex<AssetStore>,
    pub library: Library,
    /// Polices fournies et polices de `fonts_dir`, relues après chaque ajout ou suppression.
    pub fonts: RwLock<FontSet>,
    pub fonts_dir: PathBuf,
    /// Polices installées sur le poste, lues une fois au démarrage (en arrière-plan).
    system_fonts: RwLock<Vec<FontFace>>,
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
            system_fonts: RwLock::new(Vec::new()),
            pending_fonts: Mutex::new(Vec::new()),
        }
    }

    pub fn reload_fonts(&self) {
        let set = FontSet::with_user_dir(&self.fonts_dir).with_system(&self.system_fonts.read().unwrap());
        *self.fonts.write().unwrap() = set;
    }

    pub fn set_system_fonts(&self, faces: Vec<FontFace>) {
        *self.system_fonts.write().unwrap() = faces;
        self.reload_fonts();
    }

    /// Indice d'une police du poste, pour la servir par `/sysfont/<indice>`.
    pub fn system_font_index(&self, path: &Path) -> Option<usize> {
        self.system_fonts.read().unwrap().iter().position(|f| f.system_path() == Some(path))
    }

    pub fn system_font_path(&self, index: usize) -> Option<PathBuf> {
        self.system_fonts.read().unwrap().get(index).and_then(|f| f.system_path().map(Path::to_path_buf))
    }
}
