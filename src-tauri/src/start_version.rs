//! Version du début de séance : juste avant la première écriture d'une séance dans le fichier d'un document
//! (enregistrement automatique ou à la main), le fichier tel qu'il était est copié dans le dossier de
//! l'application (`debut-de-seance/`), pas à côté du document. Une seule copie par document, remplacée à la
//! première écriture de la séance suivante ; Fichier > « Revenir à la version du début de séance » la reprend.
//! Les copies de plus de `MAX_AGE` jours sont effacées au lancement, et celle d'un document mis à la corbeille.
//!
//! Chaque copie : `<id>.carnet` et sa fiche `<id>.json` (le fichier d'origine, la date de la version). Le nom
//! de la copie ne dit rien du document ; la fiche se lit pour retrouver la copie d'un document.

use carnet_core::fsutil::write_atomically;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Une copie plus vieille est effacée au lancement : assez pour revenir sur un document repris d'une semaine
/// à l'autre (vacances comprises), sans garder des photos d'élèves indéfiniment.
pub const MAX_AGE: Duration = Duration::from_secs(30 * 24 * 3600);

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct StartInfo {
    pub source: String,
    /// Date de modification du fichier copié (UTC, RFC 3339) : celle de la version gardée.
    pub version_at: String,
}

fn infos(dir: &Path) -> Vec<(String, StartInfo)> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .filter_map(|p| {
            let id = p.file_stem()?.to_str()?.to_string();
            let info: StartInfo = serde_json::from_slice(&std::fs::read(&p).ok()?).ok()?;
            carnet_core::is_safe_id(&id).then_some((id, info))
        })
        .collect()
}

fn find(dir: &Path, source: &str) -> Option<(String, StartInfo)> {
    infos(dir).into_iter().find(|(id, i)| i.source == source && dir.join(format!("{id}.carnet")).is_file())
}

fn rfc3339(t: SystemTime) -> String {
    time::OffsetDateTime::from(t).format(&time::format_description::well_known::Rfc3339).unwrap_or_default()
}

/// Garde le fichier `source` tel qu'il est, à la place de sa copie précédente. Un échec laisse la précédente.
pub fn keep(dir: &Path, source: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let key = source.to_string_lossy().into_owned();
    let id = find(dir, &key).map(|(id, _)| id).unwrap_or_else(carnet_core::new_id);
    let version_at = rfc3339(std::fs::metadata(source)?.modified()?);
    let mut from = std::fs::File::open(source)?;
    write_atomically(&dir.join(format!("{id}.carnet")), |f| std::io::copy(&mut from, f).map(|_| ()))?;
    let json = serde_json::to_vec_pretty(&StartInfo { source: key, version_at }).map_err(std::io::Error::other)?;
    write_atomically(&dir.join(format!("{id}.json")), |f| f.write_all(&json))
}

/// La version gardée du document `source` : sa fiche et le chemin de la copie.
pub fn info(dir: &Path, source: &str) -> Option<(StartInfo, PathBuf)> {
    find(dir, source).map(|(id, i)| (i, dir.join(format!("{id}.carnet"))))
}

fn remove(dir: &Path, id: &str) {
    let _ = std::fs::remove_file(dir.join(format!("{id}.carnet")));
    let _ = std::fs::remove_file(dir.join(format!("{id}.json")));
}

/// Le document est mis à la corbeille : sa version gardée n'a plus de sens.
pub fn forget(dir: &Path, source: &str) {
    if let Some((id, _)) = find(dir, source) {
        remove(dir, &id);
    }
}

/// Au lancement : efface les copies écrites il y a plus de `max_age`, et les fichiers sans fiche ou sans copie.
/// Le fichier d'origine introuvable ne suffit pas (une clé USB absente aujourd'hui).
pub fn tidy(dir: &Path, now: SystemTime, max_age: Duration) -> usize {
    let mut removed = 0;
    let complete: Vec<String> = infos(dir).into_iter().map(|(id, _)| id).collect();
    let Ok(entries) = std::fs::read_dir(dir) else { return 0 };
    for path in entries.filter_map(|e| e.ok()).map(|e| e.path()) {
        let Some(id) = path.file_stem().and_then(|s| s.to_str()).map(String::from) else { continue };
        let is_copy = path.extension().is_some_and(|e| e == "carnet");
        if !is_copy {
            if !dir.join(format!("{id}.carnet")).is_file() {
                let _ = std::fs::remove_file(&path);
            }
            continue;
        }
        let old = std::fs::metadata(&path).and_then(|m| m.modified()).is_ok_and(|t| now.duration_since(t).is_ok_and(|age| age > max_age));
        if old || !complete.contains(&id) {
            remove(dir, &id);
            removed += 1;
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_age(path: &Path, t: SystemTime) {
        std::fs::File::options().write(true).open(path).unwrap().set_modified(t).unwrap();
    }

    #[test]
    fn keeps_the_file_as_it_was_then_replaces_it() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("debut-de-seance");
        let doc = root.path().join("Sortie à la ferme.carnet");
        let key = doc.to_string_lossy().into_owned();
        assert_eq!(info(&dir, &key), None);
        std::fs::write(&doc, b"version de lundi").unwrap();
        keep(&dir, &doc).unwrap();
        let (i, copy) = info(&dir, &key).unwrap();
        assert_eq!(std::fs::read(&copy).unwrap(), b"version de lundi");
        assert_eq!(i.version_at, rfc3339(std::fs::metadata(&doc).unwrap().modified().unwrap()));
        // Le document change ; la séance suivante remplace la copie : une seule par document.
        std::fs::write(&doc, b"version de mardi").unwrap();
        keep(&dir, &doc).unwrap();
        assert_eq!(std::fs::read(info(&dir, &key).unwrap().1).unwrap(), b"version de mardi");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
        // Le nom de la copie ne dit rien du document.
        assert!(std::fs::read_dir(&dir).unwrap().all(|e| !e.unwrap().file_name().to_string_lossy().contains("ferme")));
        // Un autre document a sa propre copie.
        let other = root.path().join("autre.carnet");
        std::fs::write(&other, b"autre").unwrap();
        keep(&dir, &other).unwrap();
        assert_eq!(std::fs::read(info(&dir, &key).unwrap().1).unwrap(), b"version de mardi");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 4);
    }

    #[test]
    fn a_missing_source_keeps_the_previous_copy() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("debut-de-seance");
        let doc = root.path().join("fiche.carnet");
        std::fs::write(&doc, b"lundi").unwrap();
        keep(&dir, &doc).unwrap();
        std::fs::remove_file(&doc).unwrap();
        assert!(keep(&dir, &doc).is_err());
        assert_eq!(std::fs::read(info(&dir, &doc.to_string_lossy()).unwrap().1).unwrap(), b"lundi");
    }

    #[test]
    fn old_copies_and_trashed_documents_are_cleaned() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("debut-de-seance");
        let (a, b, c) = (root.path().join("a.carnet"), root.path().join("b.carnet"), root.path().join("c.carnet"));
        for p in [&a, &b, &c] {
            std::fs::write(p, b"x").unwrap();
            keep(&dir, p).unwrap();
        }
        let now = SystemTime::now();
        let key = |p: &Path| p.to_string_lossy().into_owned();
        set_age(&info(&dir, &key(&a)).unwrap().1, now - MAX_AGE - Duration::from_secs(3600));
        set_age(&info(&dir, &key(&b)).unwrap().1, now - MAX_AGE + Duration::from_secs(3600));
        // Le document b a disparu (clé USB absente) : sa copie reste tant qu'elle est récente.
        std::fs::remove_file(&b).unwrap();
        // Une fiche orpheline (copie inachevée) est rangée aussi.
        std::fs::write(dir.join(format!("{}.json", carnet_core::new_id())), b"{}").unwrap();
        assert_eq!(tidy(&dir, now, MAX_AGE), 1);
        assert_eq!(info(&dir, &key(&a)), None);
        assert!(info(&dir, &key(&b)).is_some());
        forget(&dir, &key(&c));
        assert_eq!(info(&dir, &key(&c)), None);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
    }
}
