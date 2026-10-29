//! Reprise des données de Carnet, l'ancien nom de l'application : modèles personnels et polices ajoutées.

use std::path::Path;

/// Identifiant de l'application avant son changement de nom ; son dossier de données est voisin du nôtre.
pub const LEGACY_IDENTIFIER: &str = "fr.carnet.app";

/// Copie `modeles` et `polices` de `old` vers `new`, chacun seulement s'il manque dans `new` : le dossier
/// `new` lui-même existe déjà (Tauri y range le journal sous Linux), et `modeles` y est créé à chaque
/// lancement, donc la copie n'a lieu qu'une fois. Rien n'est déplacé ni effacé dans `old` : désinstaller
/// Carnet ne touche plus nos données. Chaque dossier passe par un dossier provisoire renommé à la fin :
/// une copie interrompue est refaite au lancement suivant. Vrai si une copie a eu lieu.
pub fn copy_legacy_data(old: &Path, new: &Path) -> std::io::Result<bool> {
    let mut copied = false;
    for sub in ["modeles", "polices"] {
        let (from, to) = (old.join(sub), new.join(sub));
        if !from.is_dir() || to.exists() {
            continue;
        }
        let partial = new.join(format!("{sub}.reprise"));
        if partial.exists() {
            std::fs::remove_dir_all(&partial)?;
        }
        copy_dir(&from, &partial)?;
        std::fs::rename(&partial, &to)?;
        copied = true;
    }
    Ok(copied)
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

    /// Sous Linux, Tauri crée le dossier (journal, données du navigateur) avant `setup`.
    #[test]
    fn data_is_copied_even_when_tauri_already_created_the_new_folder() {
        let root = tempfile::tempdir().unwrap();
        let (old, new) = (root.path().join("fr.carnet.app"), root.path().join("fr.cahutin.app"));
        write(&old.join("modeles/a.carnet-modele"), "modèle");
        write(&old.join("polices/y.ttf"), "police");
        fs::create_dir_all(new.join("logs")).unwrap();
        assert!(copy_legacy_data(&old, &new).unwrap());
        assert_eq!(fs::read_to_string(new.join("modeles/a.carnet-modele")).unwrap(), "modèle");
        assert_eq!(fs::read_to_string(new.join("polices/y.ttf")).unwrap(), "police");
    }

    /// Cahutin déjà lancé : son dossier de modèles existe (même vide), rien n'est copié ni écrasé.
    #[test]
    fn nothing_is_copied_over_existing_cahutin_folders() {
        let root = tempfile::tempdir().unwrap();
        let (old, new) = (root.path().join("fr.carnet.app"), root.path().join("fr.cahutin.app"));
        write(&old.join("modeles/a.carnet-modele"), "modèle");
        write(&old.join("polices/y.ttf"), "ancienne");
        fs::create_dir_all(new.join("modeles")).unwrap();
        write(&new.join("polices/y.ttf"), "nouvelle");
        assert!(!copy_legacy_data(&old, &new).unwrap());
        assert_eq!(fs::read_dir(new.join("modeles")).unwrap().count(), 0);
        assert_eq!(fs::read_to_string(new.join("polices/y.ttf")).unwrap(), "nouvelle");
    }

    /// Une copie interrompue laisse un dossier provisoire : le lancement suivant recommence proprement.
    #[test]
    fn an_interrupted_copy_is_retried() {
        let root = tempfile::tempdir().unwrap();
        let (old, new) = (root.path().join("fr.carnet.app"), root.path().join("fr.cahutin.app"));
        write(&old.join("modeles/a.carnet-modele"), "modèle");
        write(&new.join("modeles.reprise/a.carnet-modele"), "moitié");
        assert!(copy_legacy_data(&old, &new).unwrap());
        assert_eq!(fs::read_to_string(new.join("modeles/a.carnet-modele")).unwrap(), "modèle");
        assert!(!new.join("modeles.reprise").exists());
    }

    #[test]
    fn nothing_happens_without_an_old_folder() {
        let root = tempfile::tempdir().unwrap();
        let (old, new) = (root.path().join("fr.carnet.app"), root.path().join("fr.cahutin.app"));
        assert!(!copy_legacy_data(&old, &new).unwrap());
        assert!(!new.exists());
    }
}
