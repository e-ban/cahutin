//! État partagé : les images en mémoire et la bibliothèque de modèles. Le document vit dans l'interface.

use carnet_core::photo::AssetStore;
use carnet_core::templates::Library;
use std::sync::Mutex;

pub struct AppState {
    pub store: Mutex<AssetStore>,
    pub library: Library,
}
