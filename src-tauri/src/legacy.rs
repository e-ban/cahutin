//! Reprise des données de Carnet, l'ancien nom de l'application : modèles personnels et polices ajoutées.

use std::path::Path;

/// Identifiant de l'application avant son changement de nom ; son dossier de données est voisin du nôtre.
pub const LEGACY_IDENTIFIER: &str = "fr.carnet.app";

/// Au premier lancement (dossier `new` absent), copie `modeles` et `polices` de `old` vers `new`, sans
/// rien déplacer ni effacer : désinstaller Carnet ne touche plus nos données. Vrai si une copie a eu lieu.
pub fn copy_legacy_data(old: &Path, new: &Path) -> std::io::Result<bool> {
    if new.exists() || !old.is_dir() {
        return Ok(false);
    }
    for sub in ["modeles", "polices"] {
        if old.join(sub).is_dir() {
            copy_dir(&old.join(sub), &new.join(sub))?;
        }
    }
    Ok(true)
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn templates_and_fonts_are_copied_and_the_old_folder_is_kept() {
        let root = tempfile::tempdir().unwrap();
        let (old, new) = (root.path().join("fr.carnet.app"), root.path().join("fr.cahutin.app"));
        let files = ["modeles/a.carnet-modele", "polices/x/y.ttf", "polices/index.json"];
        for (i, f) in files.iter().enumerate() {
            write(&old.join(f), &format!("contenu {i}"));
        }
        assert!(copy_legacy_data(&old, &new).unwrap());
        for (i, f) in files.iter().enumerate() {
            assert_eq!(fs::read_to_string(new.join(f)).unwrap(), format!("contenu {i}"));
            assert!(old.join(f).exists(), "{f} doit rester dans l'ancien dossier");
        }
    }

    #[test]
    fn nothing_is_copied_when_the_new_folder_exists() {
        let root = tempfile::tempdir().unwrap();
        let (old, new) = (root.path().join("fr.carnet.app"), root.path().join("fr.cahutin.app"));
        write(&old.join("modeles/a.carnet-modele"), "modèle");
        fs::create_dir_all(&new).unwrap();
        assert!(!copy_legacy_data(&old, &new).unwrap());
        assert_eq!(fs::read_dir(&new).unwrap().count(), 0);
    }

    #[test]
    fn nothing_happens_without_an_old_folder() {
        let root = tempfile::tempdir().unwrap();
        let (old, new) = (root.path().join("fr.carnet.app"), root.path().join("fr.cahutin.app"));
        assert!(!copy_legacy_data(&old, &new).unwrap());
        assert!(!new.exists());
    }
}
