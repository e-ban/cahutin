//! Écriture sûre d'un fichier : un fichier temporaire voisin, écrit jusqu'au disque, puis renommé.
//! Un échec (ou une coupure de courant) laisse l'ancien fichier intact.

use std::fs::File;
use std::path::{Path, PathBuf};

pub(crate) fn write_atomically<E: From<std::io::Error>>(path: &Path, write: impl FnOnce(&mut File) -> Result<(), E>) -> Result<(), E> {
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
