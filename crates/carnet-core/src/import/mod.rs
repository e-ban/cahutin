//! Import de fichiers : pages d'un autre document Carnet, ou fichiers convertis par LibreOffice.

pub mod carnet;
pub mod paragraphs;

use crate::model::{ElementKind, Page};
use crate::native::NativeError;
use crate::photo::Asset;
use serde::Serialize;
use ts_rs::TS;

/// Compte rendu d'un import : ce qui a été repris, et les dessins qui ne l'ont pas été.
#[derive(Serialize, Clone, Debug, Default, PartialEq, TS)]
#[ts(export)]
pub struct ImportReport {
    pub pages: usize,
    pub texts: usize,
    pub images: usize,
    pub shapes: usize,
    pub skipped: usize,
}

impl ImportReport {
    /// Compte les pages et leurs éléments (photos et images OpenMoji ensemble).
    pub fn count(pages: &[Page]) -> ImportReport {
        let mut r = ImportReport { pages: pages.len(), ..ImportReport::default() };
        for e in pages.iter().flat_map(|p| &p.elements) {
            match e.kind {
                ElementKind::Text(_) => r.texts += 1,
                ElementKind::Photo(_) | ElementKind::Image(_) => r.images += 1,
                ElementKind::Shape(_) => r.shapes += 1,
            }
        }
        r
    }
}

/// Pages prêtes à ajouter au document, avec leurs photos et leurs polices.
pub struct Imported {
    pub pages: Vec<Page>,
    pub assets: Vec<Asset>,
    pub fonts: Vec<(String, Vec<u8>)>,
    pub report: ImportReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportError {
    NoLibreOffice,
    Unsupported,
    Conversion,
    Timeout,
    Cancelled,
    Unreadable,
    Native(NativeError),
}

impl ImportError {
    pub fn message_fr(&self) -> String {
        match self {
            ImportError::NoLibreOffice => "Pour importer ce fichier, installez LibreOffice (gratuit), puis recommencez. Les documents Carnet s'importent sans LibreOffice.".into(),
            ImportError::Unsupported => "Ce type de fichier ne peut pas être importé.".into(),
            ImportError::Conversion => "LibreOffice n'a pas pu lire ce fichier (abîmé ou protégé par un mot de passe ?).".into(),
            ImportError::Timeout => "La conversion a pris trop de temps (plus de 2 minutes).".into(),
            ImportError::Cancelled => "Import annulé.".into(),
            ImportError::Unreadable => "Le fichier converti est illisible.".into(),
            ImportError::Native(e) => e.message_fr(),
        }
    }
}
