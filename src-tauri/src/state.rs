//! État partagé : les images, la bibliothèque de modèles et les polices. Le document vit dans l'interface.

use carnet_core::fonts::{FontFace, FontSet};
use carnet_core::photo::AssetStore;
use carnet_core::templates::Library;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64};
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
    /// Demande d'annulation de l'import de fichier en cours.
    pub import_cancel: AtomicBool,
    /// Demande d'arrêt de l'assistant (bouton Arrêter).
    pub assistant_cancel: AtomicBool,
    /// Dossier des listes de classe (`Documents/Cahutin/Classes`).
    pub classes_dir: PathBuf,
    /// Photothèque ouverte (celle de la classe courante) : son dossier et son index.
    pub phototheque: Mutex<Option<(PathBuf, carnet_core::phototheque::Phototheque)>>,
    /// Le dernier dossier que l'enseignant a cessé de suivre, avec ses photos, le temps de l'avis « Annuler » :
    /// le dossier de la photothèque d'où il vient, et ce qui en a été retiré.
    pub phototheque_unfollowed: Mutex<Option<(PathBuf, carnet_core::phototheque::Unfollowed)>>,
    /// Tenu pendant un parcours des dossiers suivis : un seul à la fois.
    pub phototheque_scan: Mutex<()>,
    /// Numéro du remplissage de vignettes en cours : changé quand une autre classe s'ouvre ou qu'un
    /// nouveau remplissage commence, ce qui arrête le précédent.
    pub thumb_fill: AtomicU64,
    /// « Mes images » déjà importées : (nom, date, taille) → image, pour ne pas la copier à chaque clic.
    pub my_images: Mutex<std::collections::HashMap<(String, u64, u64), carnet_core::AssetMeta>>,
    /// Dossier de la copie de secours du document ouvert (`<données de l'application>/secours`).
    pub backup_dir: PathBuf,
}

impl AppState {
    pub fn new(store: AssetStore, library: Library, fonts_dir: PathBuf, classes_dir: PathBuf, backup_dir: PathBuf) -> AppState {
        AppState {
            store: Mutex::new(store),
            library,
            fonts: RwLock::new(FontSet::with_user_dir(&fonts_dir)),
            fonts_dir,
            system_fonts: RwLock::new(Vec::new()),
            pending_fonts: Mutex::new(Vec::new()),
            import_cancel: AtomicBool::new(false),
            assistant_cancel: AtomicBool::new(false),
            classes_dir,
            phototheque: Mutex::new(None),
            phototheque_unfollowed: Mutex::new(None),
            phototheque_scan: Mutex::new(()),
            thumb_fill: AtomicU64::new(0),
            my_images: Mutex::new(Default::default()),
            backup_dir,
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
