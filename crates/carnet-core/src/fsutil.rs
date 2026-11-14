//! Écriture sûre d'un fichier : un fichier temporaire voisin, écrit jusqu'au disque, puis renommé.
//! Un échec (ou une coupure de courant) laisse l'ancien fichier intact.

use std::fs::File;
use std::path::{Path, PathBuf};

pub fn write_atomically<E: From<std::io::Error>>(path: &Path, write: impl FnOnce(&mut File) -> Result<(), E>) -> Result<(), E> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let result = File::create(&tmp).map_err(E::from).and_then(|mut f| {
        write(&mut f)?;
        f.sync_all().map_err(E::from)
    });
    let result = result.and_then(|_| std::fs::rename(&tmp, path).map_err(E::from));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Ce qui dit qu'un fichier a changé depuis qu'on l'a lu ou écrit : sa taille et sa date de modification.
/// Pas d'empreinte du contenu : il faudrait relire tout le fichier (photos comprises) à chaque enregistrement.
/// Une fausse alerte (date touchée par un logiciel de synchronisation) ne fait que poser une question.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FileStamp {
    len: u64,
    modified: Option<std::time::SystemTime>,
}

/// La marque du fichier, ou rien s'il n'existe pas (ou n'est pas un fichier).
pub fn stamp(path: &Path) -> Option<FileStamp> {
    let m = std::fs::metadata(path).ok().filter(|m| m.is_file())?;
    Some(FileStamp { len: m.len(), modified: m.modified().ok() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn replaces_the_file_and_leaves_no_temporary() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fiche.carnet");
        std::fs::write(&path, b"ancien").unwrap();
        write_atomically::<std::io::Error>(&path, |f| f.write_all(b"nouveau")).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"nouveau");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    /// Une écriture qui échoue en route (disque plein, clé retirée) : l'ancien fichier reste entier.
    #[test]
    fn a_failed_write_keeps_the_old_file_whole() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fiche.carnet");
        std::fs::write(&path, b"ancien").unwrap();
        let r = write_atomically(&path, |f| {
            f.write_all(b"moit")?;
            Err(std::io::Error::other("coupure"))
        });
        assert!(r.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"ancien");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    /// Le fichier est encore ouvert en lecture ailleurs dans l'application. Sous Windows, le remplacement d'un
    /// fichier ouvert dépend du partage choisi à l'ouverture : ce dont l'enregistrement a besoin, c'est que le
    /// fichier soit soit remplacé, soit laissé entier, et jamais de fichier temporaire oublié à côté.
    #[test]
    fn a_file_still_open_for_reading_is_replaced_or_left_whole() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fiche.carnet");
        std::fs::write(&path, b"ancien").unwrap();
        let mut reader = File::open(&path).unwrap();
        let result = write_atomically::<std::io::Error>(&path, |f| f.write_all(b"nouveau"));
        let mut old = Vec::new();
        reader.read_to_end(&mut old).unwrap();
        drop(reader);
        let expected: &[u8] = if result.is_ok() { b"nouveau" } else { b"ancien" };
        assert_eq!(std::fs::read(&path).unwrap(), expected);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn the_stamp_changes_with_the_size_or_the_date() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fiche.carnet");
        assert_eq!(stamp(&path), None);
        assert_eq!(stamp(dir.path()), None);
        std::fs::write(&path, b"abc").unwrap();
        let first = stamp(&path).unwrap();
        assert_eq!(stamp(&path), Some(first));
        let t = std::time::SystemTime::now() + std::time::Duration::from_secs(60);
        File::options().write(true).open(&path).unwrap().set_modified(t).unwrap();
        assert_ne!(stamp(&path), Some(first), "touché : même taille, autre date");
        let touched = stamp(&path).unwrap();
        std::fs::write(&path, b"abcd").unwrap();
        File::options().write(true).open(&path).unwrap().set_modified(t).unwrap();
        assert_ne!(stamp(&path), Some(touched), "même date, autre taille");
    }
}
