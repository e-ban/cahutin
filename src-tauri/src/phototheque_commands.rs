//! Commandes de la photothèque de la classe : un dossier par classe, à côté du dossier des classes
//! (`Documents/Cahutin/Photos/<nom du fichier de la classe>/`), qui garde l'index et le cache de vignettes.
//! Les photos restent dans les dossiers que l'enseignant suit : rien n'est copié. Une seule photothèque
//! ouverte à la fois, celle de la classe courante. Le journal ne contient que des nombres.

use crate::state::AppState;
use carnet_core::AssetMeta;
use carnet_core::class::CLASS_EXTENSION;
use carnet_core::phototheque::originals::{self, Match, OriginalsReport};
use carnet_core::phototheque::{self, ClassPhoto, FollowedFolder, Phototheque, ScanReport, TagChange, thumbs};
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use ts_rs::TS;

const MISSING_FILE: &str = "Cette photo est introuvable : la clé USB est-elle branchée ?";

/// Dossier des photos d'une classe : seulement pour un fichier de classe du dossier des classes.
pub fn library_dir(classes_dir: &Path, class_path: &Path) -> Result<PathBuf, String> {
    let inside = class_path.parent().zip(classes_dir.canonicalize().ok()).is_some_and(|(p, d)| p.canonicalize().ok() == Some(d));
    let stem = class_path.file_stem().and_then(|s| s.to_str()).filter(|_| class_path.extension().and_then(|e| e.to_str()) == Some(CLASS_EXTENSION));
    match (inside, stem) {
        (true, Some(stem)) => Ok(classes_dir.parent().unwrap_or(classes_dir).join("Photos").join(stem)),
        _ => Err("Cette classe n'a pas de photothèque.".into()),
    }
}

fn with_open<T>(state: &AppState, f: impl FnOnce(&Path, &mut Phototheque) -> Result<T, String>) -> Result<T, String> {
    let mut lib = state.phototheque.lock().unwrap();
    let Some((dir, index)) = lib.as_mut() else { return Err("Aucune photothèque n'est ouverte.".into()) };
    f(dir, index)
}

/// Le dossier et une copie de l'index ouvert, pour un long travail fait sans tenir le verrou.
fn snapshot(state: &AppState) -> Result<(PathBuf, Phototheque), String> {
    with_open(state, |dir, index| Ok((dir.to_path_buf(), index.clone())))
}

/// Ouvre la photothèque de la classe et la rend tout de suite, telle que l'index la connaît : le disque
/// n'est parcouru qu'ensuite (`phototheque_scan`).
pub fn open_in(state: &AppState, class_path: &Path) -> Result<Phototheque, String> {
    let dir = library_dir(&state.classes_dir, class_path)?;
    let index = phototheque::open(&dir)?;
    log::info!("phototheque_open photos={} folders={}", index.photos.len(), index.folders.len());
    let mut open = state.phototheque.lock().unwrap();
    // Une autre classe : le remplissage des vignettes de la précédente s'arrête.
    if open.as_ref().is_none_or(|(d, _)| *d != dir) {
        state.thumb_fill.fetch_add(1, Ordering::SeqCst);
    }
    *open = Some((dir, index.clone()));
    Ok(index)
}

#[tauri::command]
pub fn phototheque_open(state: State<'_, AppState>, class_path: String) -> Result<Phototheque, String> {
    open_in(&state, Path::new(&class_path))
}

/// Ce que le parcours annonce à l'interface.
#[derive(Serialize, Clone, Debug, TS)]
#[ts(export)]
#[serde(tag = "type")]
pub enum ScanEvent {
    /// Photos nouvelles, retrouvées ailleurs, changées ou de retour, par paquets de 50, telles qu'elles
    /// sont dans l'index (avec les étiquettes posées pendant le parcours).
    Photos { photos: Vec<ClassPhoto> },
    /// Fin du parcours. L'index entier est joint : les photos devenues absentes ne passent pas par les
    /// paquets, l'interface remplace donc sa liste par celle-ci, sans rappeler `phototheque_open`.
    Done { report: ScanReport, phototheque: Phototheque },
}

/// Reporte des résultats du parcours dans l'index du dossier `dir`, puis l'enregistre : l'index ouvert,
/// ou celui du disque si l'on a changé de classe pendant le parcours. Rend les photos reportées.
fn merge_saved(state: &AppState, dir: &Path, photos: &[ClassPhoto], folders: &[FollowedFolder]) -> Result<Vec<ClassPhoto>, String> {
    let mut open = state.phototheque.lock().unwrap();
    match open.as_mut() {
        Some((d, index)) if d == dir => {
            let merged = phototheque::merge_scan(index, photos, folders);
            phototheque::save(dir, index)?;
            Ok(merged)
        }
        _ => {
            let mut index = phototheque::open(dir)?;
            let merged = phototheque::merge_scan(&mut index, photos, folders);
            phototheque::save(dir, &index)?;
            Ok(merged)
        }
    }
}

/// Parcourt les dossiers suivis sur une copie de l'index, sans tenir le verrou : l'enseignant peut
/// étiqueter pendant ce temps. Chaque paquet rejoint l'index, enregistré, puis il est annoncé.
pub fn scan_in(state: &AppState, on_event: impl Fn(ScanEvent)) -> Result<ScanReport, String> {
    // Un seul parcours à la fois : deux parcours donneraient deux identifiants au même fichier nouveau.
    let _one = state.phototheque_scan.lock().unwrap_or_else(|e| e.into_inner());
    let (dir, mut scanned) = snapshot(state)?;
    let mut failed = None;
    let report = phototheque::scan(&mut scanned, &dir, &mut |batch| {
        if failed.is_none() {
            match merge_saved(state, &dir, batch, &[]) {
                Ok(photos) => on_event(ScanEvent::Photos { photos }),
                Err(e) => failed = Some(e),
            }
        }
    });
    if let Some(e) = failed {
        return Err(e);
    }
    merge_saved(state, &dir, &scanned.photos, &scanned.folders)?;
    let index = match snapshot(state) {
        Ok((d, index)) if d == dir => index,
        _ => phototheque::open(&dir)?,
    };
    log::info!("phototheque_scan added={} moved={} missing={} missing_folders={} skipped={}", report.added, report.moved, report.missing, report.missing_folders.len(), report.skipped);
    on_event(ScanEvent::Done { report: report.clone(), phototheque: index });
    Ok(report)
}

/// Un long travail (parcours, recherche des originaux), fait sur un fil fait pour attendre, jamais sur les
/// fils du runtime asynchrone : sur un ordinateur de deux coeurs, un parcours et un second qui attend le
/// verrou les prendraient tous, et plus aucune commande ne répondrait. Une panique imprévue donne une erreur
/// simple : la commande répond toujours.
async fn off_runtime<T: Send + 'static>(work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work).await.unwrap_or_else(|_| {
        log::error!("phototheque_job_panicked");
        Err("Le travail sur les photos s'est arrêté. Réessayez plus tard.".into())
    })
}

/// Parcourt les dossiers suivis, puis un seul fil remplit le cache de vignettes en arrière-plan.
#[tauri::command]
pub async fn phototheque_scan(app: AppHandle, on_progress: Channel<ScanEvent>) -> Result<ScanReport, String> {
    off_runtime(move || {
        let state = app.state::<AppState>();
        let report = scan_in(&state, |e| {
            let _ = on_progress.send(e);
        })?;
        // Ce remplissage remplace celui d'un parcours précédent : jamais deux fils à la fois.
        let epoch = state.thumb_fill.fetch_add(1, Ordering::SeqCst) + 1;
        let app = app.clone();
        std::thread::spawn(move || fill_thumbs(&app.state::<AppState>(), epoch));
        Ok(report)
    })
    .await
}

/// Fait les vignettes manquantes de la photothèque ouverte, une photo après l'autre, et efface d'abord
/// celles qui ne servent plus (fichiers du cache de Cahutin, jamais ceux de l'enseignant). S'arrête dès
/// que `thumb_fill` change : une autre classe ouverte ou un nouveau remplissage.
pub fn fill_thumbs(state: &AppState, epoch: u64) {
    fill_thumbs_with(state, epoch, thumbs::thumb);
}

/// `fill_thumbs`, avec la fabrique de vignettes donnée (un test lui fait décoder une photo abîmée). Une photo
/// qui fait paniquer le décodeur est sautée et comptée : les autres se font quand même. Le verrou de la
/// photothèque n'est jamais tenu pendant une vignette.
fn fill_thumbs_with(state: &AppState, epoch: u64, make: fn(&Path, &ClassPhoto) -> Result<PathBuf, carnet_core::photo::ImportError>) {
    let current = || state.thumb_fill.load(Ordering::SeqCst) == epoch;
    if !current() {
        return;
    }
    let Ok((dir, lib)) = snapshot(state) else { return };
    if !current() {
        return;
    }
    // Un fichier tout récent peut être une vignette en cours d'écriture (protocole) : laissé pour plus tard.
    let recent = |f: &Path| std::fs::metadata(f).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_none_or(|age| age.as_secs() < 60);
    for stale in thumbs::stale_thumbs(&dir, &lib).into_iter().filter(|f| !recent(f)) {
        let _ = std::fs::remove_file(stale);
    }
    let (mut made, mut failed) = (0, 0);
    for p in lib.photos.iter().filter(|p| !p.hidden && !p.missing) {
        if !current() {
            break;
        }
        match std::panic::catch_unwind(|| make(&dir, p)) {
            Ok(Ok(_)) => made += 1,
            _ => failed += 1,
        }
    }
    log::info!("phototheque_thumbs photos={made} failed={failed}");
}

/// Suit un dossier. Si c'est celui que « Ne plus suivre » vient de retirer (gardé pour « Annuler »), ses
/// photos reviennent avec leurs élèves, activités et masquage, au lieu de revenir neuves au parcours.
pub fn follow_in(state: &AppState, folder: &str) -> Result<Phototheque, String> {
    with_open(state, |dir, index| {
        let mut kept = state.phototheque_unfollowed.lock().unwrap();
        let undo = kept.as_ref().filter(|(d, r)| d == dir && Path::new(r.path()) == Path::new(folder)).map(|(_, r)| r.clone());
        let mut followed = index.clone();
        // Les mêmes refus que pour un dossier neuf (introuvable, déjà suivi, dans un dossier suivi).
        phototheque::follow(&mut followed, Path::new(folder))?;
        let restored = undo.is_some();
        if let Some(removed) = undo {
            // Le dossier reprend sa place d'avant, avec ses photos.
            followed.folders.pop();
            phototheque::refollow(&mut followed, removed);
        }
        phototheque::save(dir, &followed)?;
        *index = followed;
        if restored {
            *kept = None;
        }
        log::info!("phototheque_follow folders={}", index.folders.len());
        Ok(index.clone())
    })
}

#[tauri::command(async)]
pub fn phototheque_follow(state: State<'_, AppState>, folder: String) -> Result<Phototheque, String> {
    follow_in(&state, &folder)
}

/// « Ne plus suivre » : le dossier et ses photos sont oubliés, aucun fichier n'est touché. Ce qui est retiré
/// est gardé en mémoire pour « Annuler » (`refollow_in`), jusqu'au prochain « Ne plus suivre ».
pub fn unfollow_in(state: &AppState, folder: &str) -> Result<Phototheque, String> {
    with_open(state, |dir, index| {
        let removed = phototheque::unfollow(index, folder);
        phototheque::save(dir, index)?;
        // Un dossier qui n'était pas suivi ne retire rien : ce que garde « Annuler » pour un autre reste.
        if let Some(r) = removed {
            *state.phototheque_unfollowed.lock().unwrap() = Some((dir.to_path_buf(), r));
        }
        log::info!("phototheque_unfollow folders={} photos={}", index.folders.len(), index.photos.len());
        Ok(index.clone())
    })
}

#[tauri::command(async)]
pub fn phototheque_unfollow(state: State<'_, AppState>, folder: String) -> Result<Phototheque, String> {
    unfollow_in(&state, &folder)
}

/// « Annuler » après « Ne plus suivre » : le dossier revient avec ses photos, leurs élèves et activités.
pub fn refollow_in(state: &AppState, folder: &str) -> Result<Phototheque, String> {
    with_open(state, |dir, index| {
        let mut kept = state.phototheque_unfollowed.lock().unwrap();
        let Some((_, removed)) = kept.as_ref().filter(|(d, r)| d == dir && r.path() == folder) else {
            // Déjà suivi de nouveau (« Ajouter un dossier » l'a remis avec ses photos) : rien à faire.
            if index.folders.iter().any(|f| Path::new(&f.path) == Path::new(folder)) {
                return Ok(index.clone());
            }
            return Err("Ce dossier ne peut plus être remis : suivez-le de nouveau avec « Ajouter un dossier ».".into());
        };
        let mut restored = index.clone();
        phototheque::refollow(&mut restored, removed.clone());
        phototheque::save(dir, &restored)?;
        *index = restored;
        *kept = None;
        log::info!("phototheque_refollow folders={} photos={}", index.folders.len(), index.photos.len());
        Ok(index.clone())
    })
}

#[tauri::command(async)]
pub fn phototheque_refollow(state: State<'_, AppState>, folder: String) -> Result<Phototheque, String> {
    refollow_in(&state, &folder)
}

/// Les photos `ids` telles qu'elles sont maintenant dans l'index : la réponse d'un clic, sans tout l'index.
fn changed(index: &Phototheque, ids: &[String]) -> Vec<ClassPhoto> {
    let ids: HashSet<&String> = ids.iter().collect();
    index.photos.iter().filter(|p| ids.contains(&p.id)).cloned().collect()
}

/// « Retirer » (et son annulation) : la photo est masquée, gardée dans l'index avec ses étiquettes. Rend
/// les photos changées.
pub fn hide_in(state: &AppState, ids: &[String], hidden: bool) -> Result<Vec<ClassPhoto>, String> {
    with_open(state, |dir, index| {
        phototheque::hide(index, ids, hidden);
        phototheque::save(dir, index)?;
        log::info!("phototheque_hide photos={} hidden={hidden}", ids.len());
        Ok(changed(index, ids))
    })
}

#[tauri::command(async)]
pub fn phototheque_hide(state: State<'_, AppState>, ids: Vec<String>, hidden: bool) -> Result<Vec<ClassPhoto>, String> {
    hide_in(&state, &ids, hidden)
}

/// Élèves et activités de plusieurs photos : l'index est enregistré, seules les photos changées sont rendues.
pub fn tag_in(state: &AppState, ids: &[String], change: &TagChange) -> Result<Vec<ClassPhoto>, String> {
    with_open(state, |dir, index| {
        phototheque::tag(index, ids, change);
        phototheque::save(dir, index)?;
        Ok(changed(index, ids))
    })
}

#[tauri::command(async)]
pub fn phototheque_tag(state: State<'_, AppState>, ids: Vec<String>, change: TagChange) -> Result<Vec<ClassPhoto>, String> {
    tag_in(&state, &ids, &change)
}

/// Les photos choisies, prêtes pour le document (copiées dedans, réduites) : elles rejoignent les images de
/// l'application. Si un fichier manque, rien n'est ajouté.
pub fn use_in(state: &AppState, ids: &[String]) -> Result<Vec<AssetMeta>, String> {
    let photos: Vec<ClassPhoto> = with_open(state, |_, index| Ok(index.photos.iter().filter(|p| ids.contains(&p.id)).cloned().collect()))?;
    // Les photos sont lues sans tenir le verrou de la photothèque.
    let assets = photos
        .iter()
        .map(|p| {
            if !Path::new(&p.path).is_file() {
                return Err(MISSING_FILE.to_string());
            }
            thumbs::load_asset(p).map_err(|e| e.message_fr())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut store = state.store.lock().unwrap();
    let mut metas = Vec::new();
    for a in assets {
        metas.push(a.meta.clone());
        store.insert(a).map_err(|e| e.message_fr())?;
    }
    log::info!("phototheque_use photos={}", metas.len());
    Ok(metas)
}

#[tauri::command(async)]
pub fn phototheque_use(state: State<'_, AppState>, ids: Vec<String>) -> Result<Vec<AssetMeta>, String> {
    use_in(&state, &ids)
}

/// Migration : pour chaque copie faite par l'ancienne photothèque (dans le dossier de la classe), son
/// original parmi les photos des autres dossiers suivis. Rien n'est touché ; seules les vignettes se font.
pub fn find_originals_in(state: &AppState) -> Result<OriginalsReport, String> {
    let (dir, lib) = snapshot(state)?;
    let report = originals::find_originals(&lib, &dir, &dir.to_string_lossy());
    log::info!("phototheque_find_originals matches={} unmatched={}", report.matches.len(), report.unmatched.len());
    Ok(report)
}

#[tauri::command]
pub async fn phototheque_find_originals(app: AppHandle) -> Result<OriginalsReport, String> {
    off_runtime(move || find_originals_in(&app.state::<AppState>())).await
}

/// « Non merci » : l'enseignant refuse la migration ; le bandeau ne revient plus, le lien « Retrouver les
/// originaux » reste dans la photothèque. Rien n'est touché.
pub fn settle_migration_in(state: &AppState) -> Result<Phototheque, String> {
    with_open(state, |dir, index| {
        index.migration_settled = true;
        phototheque::save(dir, index)?;
        Ok(index.clone())
    })
}

#[tauri::command(async)]
pub fn phototheque_settle_migration(state: State<'_, AppState>) -> Result<Phototheque, String> {
    settle_migration_in(&state)
}

/// Ce que la fin de la migration a fait.
#[derive(Serialize, Clone, Debug, TS)]
#[ts(export)]
pub struct OriginalsApplied {
    pub phototheque: Phototheque,
    /// Copies restées en place : la corbeille les a refusées. Elles restent montrées, avec leurs élèves et
    /// activités, pour une prochaine fois.
    #[ts(type = "number")]
    pub not_trashed: usize,
}

/// Fin de la migration, après confirmation : chaque copie rapprochée va à la corbeille (`to_trash`, une à une) ;
/// pour chaque copie partie (ou déjà absente), ses élèves et activités passent sur son original et elle est
/// masquée. Une copie que la corbeille refuse ne change pas : rien n'est laissé à moitié.
pub fn apply_originals_in(state: &AppState, matches: &[Match], mut to_trash: impl FnMut(&Path) -> Result<(), ()>) -> Result<OriginalsApplied, String> {
    with_open(state, |dir, index| {
        // Seules les copies faites par Cahutin (`<id>.<ext>` dans le dossier de la classe) peuvent partir à la
        // corbeille, jamais une photo de l'enseignant.
        let copies: Vec<(&Match, PathBuf)> = matches
            .iter()
            .filter_map(|m| index.photos.iter().find(|p| p.id == m.copy && originals::is_copy(p, dir)).map(|p| (m, PathBuf::from(&p.path))))
            .collect();
        let (mut gone, mut not_trashed) = (vec![], 0);
        for (m, file) in copies {
            if !file.exists() || to_trash(&file).is_ok() {
                gone.push(m.clone());
            } else {
                not_trashed += 1;
            }
        }
        originals::apply_originals(index, &gone);
        // L'enseignant a dit oui et des copies sont parties : la question est tranchée (le lien reste pour les
        // copies restées). Si la corbeille les a toutes refusées, rien n'a changé : la question reste ouverte.
        if !gone.is_empty() {
            index.migration_settled = true;
        }
        phototheque::save(dir, index)?;
        log::info!("phototheque_apply_originals trashed={} not_trashed={not_trashed}", gone.len());
        Ok(OriginalsApplied { phototheque: index.clone(), not_trashed })
    })
}

#[tauri::command(async)]
pub fn phototheque_apply_originals(state: State<'_, AppState>, matches: Vec<Match>) -> Result<OriginalsApplied, String> {
    apply_originals_in(&state, &matches, |file| {
        // À la corbeille de l'ordinateur, comme un document ou une classe : une erreur se rattrape.
        trash::delete(file).map_err(|e| log::error!("phototheque_trash_failed kind={}", crate::commands::trash_error_kind(&e)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use carnet_core::phototheque::INDEX_FILE;
    use carnet_core::templates::Library;
    use image::{ImageFormat, Rgb, RgbImage};
    use std::io::Cursor;
    use std::sync::Mutex;
    use std::sync::atomic::Ordering;

    fn state(dir: &Path) -> AppState {
        let classes = dir.join("Cahutin").join("Classes");
        std::fs::create_dir_all(&classes).unwrap();
        AppState::new(Default::default(), Library::open(dir.join("modeles")).unwrap(), dir.join("polices"), classes, dir.join("secours"))
    }

    /// Une classe ouverte, et sa photothèque (vide).
    fn class(s: &AppState, name: &str) -> PathBuf {
        let class = s.classes_dir.join(format!("{name}.cahutin-classe"));
        std::fs::write(&class, b"").unwrap();
        open_in(s, &class).unwrap();
        class
    }

    /// Une photo générée de 1200 x 900 dont les bandes dépendent de `seed`.
    fn pattern(seed: u32) -> RgbImage {
        let mut state = seed.wrapping_mul(2_654_435_761).wrapping_add(12345);
        let mut bands = [0u8; 12];
        for b in &mut bands {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *b = (state >> 24) as u8;
        }
        RgbImage::from_fn(1200, 900, |x, y| {
            let v = bands[(x / 100) as usize];
            let dark = if (y / 150 + x / 100) % 2 == 0 { 0 } else { 40 };
            Rgb([v.saturating_sub(dark), v / 2 + 60, 255 - v])
        })
    }

    fn jpeg(path: &Path, seed: u32) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut out = Cursor::new(Vec::new());
        pattern(seed).write_to(&mut out, ImageFormat::Jpeg).unwrap();
        std::fs::write(path, out.into_inner()).unwrap();
    }

    /// Parcourt et rend les photos annoncées par paquets, et l'index joint à la fin.
    fn scan(s: &AppState) -> (ScanReport, Vec<ClassPhoto>, Phototheque) {
        let events = Mutex::new(vec![]);
        let report = scan_in(s, |e| events.lock().unwrap().push(e)).unwrap();
        let mut photos = vec![];
        let mut done = None;
        for e in events.into_inner().unwrap() {
            match e {
                ScanEvent::Photos { photos: p } => photos.extend(p),
                ScanEvent::Done { report: r, phototheque } => {
                    assert_eq!(r, report);
                    done = Some(phototheque);
                }
            }
        }
        (report, photos, done.expect("Done arrive à la fin"))
    }

    /// Une classe qui suit un dossier `Sortie` de trois photos, parcouru.
    fn followed_three(dir: &Path) -> (AppState, PathBuf, Phototheque) {
        let s = state(dir);
        let class = class(&s, "MS");
        let folder = dir.join("Sortie");
        for i in 0..3 {
            jpeg(&folder.join(format!("p{i}.jpg")), i);
        }
        follow_in(&s, &folder.to_string_lossy()).unwrap();
        let (_, _, lib) = scan(&s);
        (s, class, lib)
    }

    #[test]
    fn a_class_gets_its_own_folder_of_photos() {
        let dir = tempfile::tempdir().unwrap();
        let s = state(dir.path());
        let class = s.classes_dir.join("MS 2026-2027.cahutin-classe");
        std::fs::write(&class, b"").unwrap();
        assert_eq!(library_dir(&s.classes_dir, &class).unwrap(), dir.path().join("Cahutin").join("Photos").join("MS 2026-2027"));
        // Ailleurs que dans le dossier des classes, ou un autre fichier : refusé.
        assert!(library_dir(&s.classes_dir, &dir.path().join("MS.cahutin-classe")).is_err());
        assert!(library_dir(&s.classes_dir, &s.classes_dir.join("notes.txt")).is_err());
    }

    #[test]
    fn a_followed_folder_of_three_photos_announces_three_photos() {
        let dir = tempfile::tempdir().unwrap();
        let s = state(dir.path());
        let class = class(&s, "MS");
        let folder = dir.path().join("Sortie");
        for i in 0..3 {
            jpeg(&folder.join(format!("p{i}.jpg")), i);
        }
        let lib = follow_in(&s, &folder.to_string_lossy()).unwrap();
        assert_eq!(lib.folders.len(), 1);
        assert!(follow_in(&s, &folder.to_string_lossy()).is_err(), "déjà suivi");
        let (report, photos, index) = scan(&s);
        assert_eq!((report.added, report.missing), (3, 0));
        assert_eq!(photos.len(), 3);
        assert_eq!(index.photos.len(), 3);
        // L'index est enregistré : rouverte, la photothèque a ses trois photos, sans rien copier.
        assert_eq!(open_in(&s, &class).unwrap().photos.len(), 3);
        assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 3);
        let lib = unfollow_in(&s, &folder.to_string_lossy()).unwrap();
        assert!(lib.folders.is_empty() && lib.photos.is_empty());
        assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 3, "rien n'est touché sur le disque");
    }

    /// « Ne plus suivre » puis « Annuler » : l'index revient exactement, enregistré ; une seule fois.
    #[test]
    fn unfollowing_a_folder_can_be_undone_exactly() {
        let dir = tempfile::tempdir().unwrap();
        let (s, class, lib) = followed_three(dir.path());
        let folder = dir.path().join("Sortie").to_string_lossy().into_owned();
        hide_in(&s, &[lib.photos[1].id.clone()], true).unwrap();
        let before = with_open(&s, |dir, index| {
            phototheque::tag(index, &[index.photos[0].id.clone()], &TagChange { add_tags: vec!["Ferme".into()], ..Default::default() });
            phototheque::save(dir, index)?;
            Ok(index.clone())
        })
        .unwrap();
        assert!(unfollow_in(&s, &folder).unwrap().photos.is_empty());
        assert_eq!(refollow_in(&s, &folder).unwrap(), before);
        assert_eq!(open_in(&s, &class).unwrap(), before, "enregistré");
        assert_eq!(refollow_in(&s, &folder).unwrap(), before, "une seconde fois : rien ne change, rien en double");
        assert!(refollow_in(&s, &dir.path().join("Jamais suivi").to_string_lossy()).is_err(), "rien à remettre");
    }

    /// « Ne plus suivre », puis suivre de nouveau le même dossier au lieu d'« Annuler » : les photos reviennent
    /// avec leurs élèves, activités et masquage, sans double ; « Annuler » n'a plus rien à remettre et ne se
    /// plaint pas.
    #[test]
    fn following_again_a_folder_just_unfollowed_brings_back_its_tags() {
        let dir = tempfile::tempdir().unwrap();
        let (s, class, lib) = followed_three(dir.path());
        let folder = dir.path().join("Sortie").to_string_lossy().into_owned();
        let (a, b) = (lib.photos[0].id.clone(), lib.photos[1].id.clone());
        tag_in(&s, std::slice::from_ref(&a), &TagChange { add_students: vec!["lea".into()], add_tags: vec!["Ferme".into()], ..Default::default() }).unwrap();
        hide_in(&s, std::slice::from_ref(&b), true).unwrap();
        let before = with_open(&s, |_, i| Ok(i.clone())).unwrap();
        unfollow_in(&s, &folder).unwrap();
        // Le même dossier, écrit avec une barre finale : c'est le même chemin.
        follow_in(&s, &format!("{folder}/")).unwrap();
        let (_, _, after) = scan(&s);
        assert_eq!(after.photos.len(), 3, "aucun double");
        assert_eq!(after, before);
        assert_eq!(open_in(&s, &class).unwrap(), before, "enregistré");
        assert_eq!(refollow_in(&s, &folder).unwrap(), before, "« Annuler » resté à l'écran ne change rien");
        // Plus rien de gardé : ne plus suivre de nouveau, puis suivre, donne un dossier neuf.
        assert!(s.phototheque_unfollowed.lock().unwrap().is_none());
    }

    /// Suivre de nouveau garde les refus de « Ajouter un dossier » : un dossier disparu n'est pas remis.
    #[test]
    fn following_again_a_folder_gone_from_the_disk_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let (s, _class, _) = followed_three(dir.path());
        let folder = dir.path().join("Sortie");
        unfollow_in(&s, &folder.to_string_lossy()).unwrap();
        std::fs::rename(&folder, dir.path().join("Ailleurs")).unwrap();
        assert!(follow_in(&s, &folder.to_string_lossy()).is_err());
        assert!(s.phototheque_unfollowed.lock().unwrap().is_some(), "« Annuler » reste possible");
    }

    /// « Ne plus suivre » un dossier qui n'est pas suivi n'efface pas ce que garde « Annuler » pour un autre.
    #[test]
    fn unfollowing_a_folder_not_followed_keeps_the_undo_of_another() {
        let dir = tempfile::tempdir().unwrap();
        let (s, _class, before) = followed_three(dir.path());
        let folder = dir.path().join("Sortie").to_string_lossy().into_owned();
        unfollow_in(&s, &folder).unwrap();
        unfollow_in(&s, &dir.path().join("Jamais suivi").to_string_lossy()).unwrap();
        assert_eq!(refollow_in(&s, &folder).unwrap(), before);
    }

    #[test]
    fn photos_that_became_missing_come_with_the_done_index() {
        let dir = tempfile::tempdir().unwrap();
        let (s, _class, _) = followed_three(dir.path());
        std::fs::remove_file(dir.path().join("Sortie/p0.jpg")).unwrap();
        let (report, photos, index) = scan(&s);
        assert_eq!(report.missing, 1);
        assert!(photos.is_empty(), "une photo devenue absente ne passe pas par les paquets");
        assert_eq!(index.photos.iter().filter(|p| p.missing).count(), 1);
    }

    #[test]
    fn a_hidden_photo_stays_hidden_after_reopening() {
        let dir = tempfile::tempdir().unwrap();
        let (s, class, lib) = followed_three(dir.path());
        let id = lib.photos[0].id.clone();
        assert!(hide_in(&s, std::slice::from_ref(&id), true).unwrap()[0].hidden);
        let lib = open_in(&s, &class).unwrap();
        assert!(lib.photos.iter().find(|p| p.id == id).unwrap().hidden);
        scan(&s);
        assert!(open_in(&s, &class).unwrap().photos.iter().find(|p| p.id == id).unwrap().hidden, "même après un parcours");
        assert!(!hide_in(&s, &[id], false).unwrap()[0].hidden);
    }

    #[test]
    fn a_used_photo_keeps_its_id_and_stays_in_the_store_once_hidden() {
        let dir = tempfile::tempdir().unwrap();
        let (s, _class, lib) = followed_three(dir.path());
        let id = lib.photos[0].id.clone();
        let metas = use_in(&s, std::slice::from_ref(&id)).unwrap();
        assert_eq!(metas.len(), 1);
        assert_eq!(metas[0].id, id);
        // Déjà posée dans un document : le magasin d'images la sert encore après « Retirer ».
        hide_in(&s, std::slice::from_ref(&id), true).unwrap();
        assert!(s.store.lock().unwrap().get(&id).is_some());
    }

    #[test]
    fn using_a_photo_whose_file_is_gone_says_so_plainly() {
        let dir = tempfile::tempdir().unwrap();
        let (s, _class, lib) = followed_three(dir.path());
        std::fs::remove_file(&lib.photos[1].path).unwrap();
        let ids = [lib.photos[0].id.clone(), lib.photos[1].id.clone()];
        assert_eq!(use_in(&s, &ids).unwrap_err(), "Cette photo est introuvable : la clé USB est-elle branchée ?");
        assert!(s.store.lock().unwrap().get(&ids[0]).is_none(), "rien n'est ajouté à moitié");
    }

    #[test]
    fn the_thumbnail_filler_makes_every_thumbnail_and_stops_for_another_class() {
        let dir = tempfile::tempdir().unwrap();
        let (s, class, lib) = followed_three(dir.path());
        let thumbs = library_dir(&s.classes_dir, &class).unwrap().join(".vignettes");
        let count = || std::fs::read_dir(&thumbs).map_or(0, |d| d.count());
        // Une autre classe ouverte : le remplissage commencé pour la première s'arrête.
        let epoch = s.thumb_fill.load(Ordering::SeqCst);
        let other = self::class(&s, "GS");
        fill_thumbs(&s, epoch);
        assert_eq!(count(), 0);
        // Rouvrir la même classe n'arrête rien ; un nouveau remplissage remplace le précédent.
        open_in(&s, &class).unwrap();
        let epoch = s.thumb_fill.load(Ordering::SeqCst);
        open_in(&s, &class).unwrap();
        assert_eq!(s.thumb_fill.load(Ordering::SeqCst), epoch);
        fill_thumbs(&s, epoch);
        assert_eq!(count(), lib.photos.len());
        assert!(library_dir(&s.classes_dir, &other).unwrap().join(".vignettes").read_dir().is_err());
        // Une vignette qui ne sert plus est effacée ; un fichier tout récent (en cours d'écriture) est laissé.
        let (old, fresh) = (thumbs.join("ancienne-1-2.jpg"), thumbs.join("x-1-2.jpg.9-0.tmp"));
        std::fs::write(&old, b"x").unwrap();
        std::fs::write(&fresh, b"x").unwrap();
        let long_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        std::fs::File::options().write(true).open(&old).unwrap().set_modified(long_ago).unwrap();
        fill_thumbs(&s, epoch);
        assert!(!old.exists() && fresh.exists());
        assert_eq!(count(), lib.photos.len() + 1);
    }

    /// Des parcours qui attendent le verrou (un second parcours, une autre classe) ne prennent aucun fil du
    /// runtime asynchrone : les autres commandes (étiqueter, retirer, poser) répondent encore.
    #[test]
    fn long_jobs_waiting_on_a_lock_leave_the_async_runtime_free() {
        static LOCK: Mutex<()> = Mutex::new(());
        let held = LOCK.lock().unwrap();
        let jobs: Vec<_> = (0..64)
            .map(|_| {
                tauri::async_runtime::spawn(off_runtime(|| {
                    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
                    Ok(())
                }))
            })
            .collect();
        let (tx, rx) = std::sync::mpsc::channel();
        tauri::async_runtime::spawn(async move { tx.send(()).unwrap() });
        let answered = rx.recv_timeout(std::time::Duration::from_secs(3));
        drop(held);
        assert!(answered.is_ok(), "une commande rapide doit répondre pendant que les parcours attendent");
        for j in jobs {
            assert!(tauri::async_runtime::block_on(j).unwrap().is_ok());
        }
    }

    /// Un travail qui panique hors des fichiers protégés : la commande répond quand même, par une erreur simple.
    #[test]
    fn a_long_job_that_panics_still_answers() {
        let answer = tauri::async_runtime::block_on(off_runtime(|| -> Result<(), String> { panic!("imprévu") }));
        assert_eq!(answer.unwrap_err(), "Le travail sur les photos s'est arrêté. Réessayez plus tard.");
    }

    /// Une photo abîmée qui fait paniquer le décodeur : sautée, les autres vignettes se font quand même.
    #[test]
    fn a_photo_that_makes_the_thumbnail_panic_does_not_stop_the_others() {
        let dir = tempfile::tempdir().unwrap();
        let (s, class, lib) = followed_three(dir.path());
        let thumbs_dir = library_dir(&s.classes_dir, &class).unwrap().join(".vignettes");
        fn broken(dir: &Path, p: &ClassPhoto) -> Result<PathBuf, carnet_core::photo::ImportError> {
            if p.path.ends_with("p0.jpg") {
                panic!("fichier abîmé");
            }
            thumbs::thumb(dir, p)
        }
        let epoch = s.thumb_fill.load(Ordering::SeqCst);
        fill_thumbs_with(&s, epoch, broken);
        assert_eq!(std::fs::read_dir(&thumbs_dir).unwrap().count(), lib.photos.len() - 1);
        // Le verrou de la photothèque n'est pas empoisonné : elle sert encore.
        assert!(hide_in(&s, &[lib.photos[1].id.clone()], true).is_ok());
    }

    /// Étiqueter ou retirer ne renvoie que les photos changées : à 10 000 photos, tout l'index à chaque clic
    /// pèse des mégaoctets. L'index est enregistré quand même.
    #[test]
    fn tagging_or_hiding_answers_only_the_photos_changed() {
        let dir = tempfile::tempdir().unwrap();
        let (s, class, lib) = followed_three(dir.path());
        let id = lib.photos[1].id.clone();
        let tagged = tag_in(&s, std::slice::from_ref(&id), &TagChange { add_tags: vec!["Ferme".into()], ..Default::default() }).unwrap();
        assert_eq!(tagged.len(), 1);
        assert_eq!((tagged[0].id.as_str(), tagged[0].tags.as_slice()), (id.as_str(), ["Ferme".to_string()].as_slice()));
        let hidden = hide_in(&s, std::slice::from_ref(&id), true).unwrap();
        assert_eq!(hidden.len(), 1);
        assert!(hidden[0].hidden && hidden[0].tags == ["Ferme"]);
        let reopened = open_in(&s, &class).unwrap();
        assert_eq!(reopened.photos.iter().find(|p| p.id == id).unwrap(), &hidden[0], "enregistré");
    }

    /// Un enregistrement qui échoue (disque plein, fichier bloqué) laisse l'index d'avant lisible, et
    /// l'étiquette posée reste en mémoire : le prochain enregistrement la garde.
    #[test]
    fn a_failed_save_leaves_the_previous_index_and_loses_no_tag() {
        let dir = tempfile::tempdir().unwrap();
        let (s, class, lib) = followed_three(dir.path());
        let library = library_dir(&s.classes_dir, &class).unwrap();
        let before = std::fs::read(library.join(INDEX_FILE)).unwrap();
        // Le fichier temporaire ne peut pas être créé : l'enregistrement échoue avant de toucher l'index.
        let blocker = library.join(format!("{INDEX_FILE}.tmp"));
        std::fs::create_dir(&blocker).unwrap();
        let (a, b) = (lib.photos[0].id.clone(), lib.photos[1].id.clone());
        assert!(tag_in(&s, std::slice::from_ref(&a), &TagChange { add_tags: vec!["Ferme".into()], ..Default::default() }).is_err());
        assert_eq!(std::fs::read(library.join(INDEX_FILE)).unwrap(), before, "l'index d'avant est intact");
        assert!(phototheque::open(&library).is_ok());
        std::fs::remove_dir(&blocker).unwrap();
        tag_in(&s, std::slice::from_ref(&b), &TagChange { add_tags: vec!["Motricité".into()], ..Default::default() }).unwrap();
        let saved = phototheque::open(&library).unwrap();
        assert_eq!(saved.photos.iter().find(|p| p.id == a).unwrap().tags, ["Ferme"]);
        assert_eq!(saved.photos.iter().find(|p| p.id == b).unwrap().tags, ["Motricité"]);
    }

    /// Mesure (lancée à la main, `--ignored --nocapture`, de préférence `--release`) : le coût d'un clic
    /// d'étiquette sur une photothèque de 10 000 photos (enregistrement, taille de l'index, réponse envoyée).
    #[test]
    #[ignore]
    fn measure_a_tag_click_on_ten_thousand_photos() {
        let dir = tempfile::tempdir().unwrap();
        let s = state(dir.path());
        class(&s, "MS");
        with_open(&s, |_, index| {
            index.photos = (0..10_000)
                .map(|i| ClassPhoto {
                    id: format!("photo-{i:05}"),
                    width_px: 4032,
                    height_px: 3024,
                    mime: carnet_core::model::Mime::Jpeg,
                    taken: Some("2026-10-14".into()),
                    added: "2026-10-15".into(),
                    students: vec!["lea".into(), "noe".into()],
                    tags: vec!["Sortie à la ferme".into()],
                    path: format!("/home/enseignant/Images/Sortie à la ferme/IMG_{i:05}.jpg"),
                    folder: "Sortie à la ferme".into(),
                    size: 3_500_000,
                    modified: 1_790_000_000,
                    fingerprint: format!("{:x}-{:016x}", 3_500_000, i),
                    hidden: false,
                    missing: false,
                })
                .collect();
            Ok(())
        })
        .unwrap();
        let ids = vec!["photo-04242".to_string()];
        let change = TagChange { add_tags: vec!["Motricité".into()], ..Default::default() };
        let runs = 20;
        let started = std::time::Instant::now();
        let mut payload = 0;
        for _ in 0..runs {
            let answer = tag_in(&s, &ids, &change).unwrap();
            payload = serde_json::to_vec(&answer).unwrap().len();
        }
        let per_click = started.elapsed() / runs;
        let dir_lib = library_dir(&s.classes_dir, &s.classes_dir.join("MS.cahutin-classe")).unwrap();
        let index_size = std::fs::metadata(dir_lib.join(INDEX_FILE)).unwrap().len();
        let save_started = std::time::Instant::now();
        for _ in 0..runs {
            with_open(&s, |dir, index| phototheque::save(dir, index)).unwrap();
        }
        let per_save = save_started.elapsed() / runs;
        println!("tag click: {per_click:?} per click, save alone {per_save:?}, index {index_size} bytes, answer {payload} bytes");
    }

    const V1: &str =r#"{"format_version": 1, "photos": [
        {"id": "c1", "width_px": 400, "height_px": 300, "mime": "Jpeg", "taken": null, "added": "2026-10-15",
         "students": ["lea"], "tags": ["Sortie à la ferme"]},
        {"id": "c2", "width_px": 400, "height_px": 300, "mime": "Jpeg", "taken": null, "added": "2026-10-16",
         "students": [], "tags": ["Motricité"]}]}"#;

    /// Une ancienne photothèque (version 1) : deux copies réduites faites par Cahutin, et leurs originaux
    /// dans un dossier `Originaux` que l'enseignant suit.
    fn old_library(dir: &Path) -> (AppState, PathBuf, PathBuf) {
        let s = state(dir);
        let library = dir.join("Cahutin").join("Photos").join("MS");
        std::fs::create_dir_all(&library).unwrap();
        for (id, seed) in [("c1", 1), ("c2", 2)] {
            let mut out = Cursor::new(Vec::new());
            pattern(seed).write_to(&mut out, ImageFormat::Jpeg).unwrap();
            let copy = carnet_core::photo::import_bytes(&out.into_inner()).unwrap();
            std::fs::write(library.join(format!("{id}.jpg")), &*copy.bytes).unwrap();
        }
        std::fs::write(library.join(INDEX_FILE), V1).unwrap();
        let originals = dir.join("Originaux");
        jpeg(&originals.join("IMG_0001.jpg"), 1);
        jpeg(&originals.join("IMG_0002.jpg"), 2);
        class(&s, "MS");
        follow_in(&s, &originals.to_string_lossy()).unwrap();
        scan(&s);
        (s, library, originals)
    }

    fn by_name<'a>(lib: &'a Phototheque, name: &str) -> &'a ClassPhoto {
        lib.photos.iter().find(|p| Path::new(&p.path).file_name().unwrap() == name).unwrap()
    }

    #[test]
    fn an_old_library_finds_its_originals_and_sends_the_copies_to_the_trash() {
        let dir = tempfile::tempdir().unwrap();
        let (s, library, _originals) = old_library(dir.path());
        let report = find_originals_in(&s).unwrap();
        assert_eq!((report.matches.len(), report.unmatched.len()), (2, 0));
        let bin = dir.path().join("Corbeille");
        let done = apply_originals_in(&s, &report.matches, |f| {
            std::fs::create_dir_all(&bin).unwrap();
            std::fs::rename(f, bin.join(f.file_name().unwrap())).unwrap();
            Ok(())
        })
        .unwrap();
        assert_eq!(done.not_trashed, 0);
        let lib = done.phototheque;
        assert!(!library.join("c1.jpg").exists() && !library.join("c2.jpg").exists(), "copies à la corbeille");
        assert!(bin.join("c1.jpg").exists() && bin.join("c2.jpg").exists());
        let o1 = by_name(&lib, "IMG_0001.jpg");
        assert_eq!((o1.students.as_slice(), o1.tags.as_slice()), (["lea".to_string()].as_slice(), ["Sortie à la ferme".to_string()].as_slice()));
        assert_eq!(by_name(&lib, "IMG_0002.jpg").tags, ["Motricité"]);
        assert!(lib.photos.iter().filter(|p| p.id == "c1" || p.id == "c2").all(|p| p.hidden));
        // Enregistré.
        assert_eq!(phototheque::open(&library).unwrap(), lib);
    }

    /// « Non merci » ou une migration faite : la question est tranchée pour de bon, même après un parcours.
    #[test]
    fn the_migration_question_once_settled_stays_settled() {
        let dir = tempfile::tempdir().unwrap();
        let (s, _library, _originals) = old_library(dir.path());
        let class = s.classes_dir.join("MS.cahutin-classe");
        assert!(!open_in(&s, &class).unwrap().migration_settled);
        assert!(settle_migration_in(&s).unwrap().migration_settled);
        scan(&s);
        assert!(open_in(&s, &class).unwrap().migration_settled, "enregistré, et gardé par le parcours");

        let dir = tempfile::tempdir().unwrap();
        let (s, _library, _originals) = old_library(dir.path());
        let report = find_originals_in(&s).unwrap();
        assert!(apply_originals_in(&s, &report.matches, |_| Ok(())).unwrap().phototheque.migration_settled);
    }

    #[test]
    fn a_failed_trash_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let (s, library, _originals) = old_library(dir.path());
        let report = find_originals_in(&s).unwrap();
        let before = with_open(&s, |_, i| Ok(i.clone())).unwrap();
        let done = apply_originals_in(&s, &report.matches, |_| Err(())).unwrap();
        assert_eq!(done.not_trashed, 2);
        assert_eq!(done.phototheque, before);
        assert_eq!(with_open(&s, |_, i| Ok(i.clone())).unwrap(), before);
        assert!(library.join("c1.jpg").exists() && library.join("c2.jpg").exists());
    }

    /// La corbeille met la première copie de côté, puis échoue : les élèves et activités de la copie partie
    /// sont sur son original ; l'autre copie reste montrée, avec les siens, pour une prochaine fois.
    #[test]
    fn a_trash_that_fails_part_way_moves_the_tags_of_the_copies_gone() {
        let dir = tempfile::tempdir().unwrap();
        let (s, library, _originals) = old_library(dir.path());
        let report = find_originals_in(&s).unwrap();
        let bin = dir.path().join("Corbeille");
        std::fs::create_dir_all(&bin).unwrap();
        let done = apply_originals_in(&s, &report.matches, |f| {
            if f.ends_with("c1.jpg") {
                std::fs::rename(f, bin.join("c1.jpg")).unwrap();
                Ok(())
            } else {
                Err(())
            }
        })
        .unwrap();
        assert_eq!(done.not_trashed, 1);
        let lib = done.phototheque;
        let o1 = by_name(&lib, "IMG_0001.jpg");
        assert_eq!((o1.students.as_slice(), o1.tags.as_slice()), (["lea".to_string()].as_slice(), ["Sortie à la ferme".to_string()].as_slice()));
        assert!(lib.photos.iter().find(|p| p.id == "c1").unwrap().hidden);
        let c2 = lib.photos.iter().find(|p| p.id == "c2").unwrap();
        assert!(!c2.hidden && c2.tags == ["Motricité"]);
        assert!(by_name(&lib, "IMG_0002.jpg").tags.is_empty());
        assert!(library.join("c2.jpg").exists());
        assert_eq!(phototheque::open(&library).unwrap(), lib, "enregistré");
    }

    #[test]
    fn a_teacher_photo_in_the_class_folder_never_goes_to_the_trash() {
        let dir = tempfile::tempdir().unwrap();
        let (s, library, originals) = old_library(dir.path());
        // Une photo de l'enseignant rangée dans le dossier de la classe, avec son double dans « Originaux ».
        jpeg(&library.join("IMG_0001.jpg"), 1);
        scan(&s);
        let lib = with_open(&s, |_, i| Ok(i.clone())).unwrap();
        let teacher = lib.photos.iter().find(|p| p.path == library.join("IMG_0001.jpg").to_string_lossy()).unwrap().id.clone();
        let report = find_originals_in(&s).unwrap();
        assert!(report.matches.iter().all(|m| m.copy != teacher) && !report.unmatched.contains(&teacher), "{report:?}");
        let original = lib.photos.iter().find(|p| p.path == originals.join("IMG_0001.jpg").to_string_lossy()).unwrap().id.clone();
        let trashed = Mutex::new(vec![]);
        apply_originals_in(&s, &[Match { copy: teacher, original }], |f| {
            trashed.lock().unwrap().push(f.to_path_buf());
            Ok(())
        })
        .unwrap();
        assert!(trashed.into_inner().unwrap().is_empty());
        assert!(library.join("IMG_0001.jpg").exists());
    }

    #[test]
    fn only_copies_of_the_class_folder_can_go_to_the_trash() {
        let dir = tempfile::tempdir().unwrap();
        let (s, _library, originals) = old_library(dir.path());
        let lib = with_open(&s, |_, i| Ok(i.clone())).unwrap();
        // Une demande venue de la page qui désigne une photo de l'enseignant comme copie : ignorée.
        let wrong = Match { copy: by_name(&lib, "IMG_0001.jpg").id.clone(), original: "c1".into() };
        let trashed = Mutex::new(vec![]);
        apply_originals_in(&s, &[wrong], |f| {
            trashed.lock().unwrap().push(f.to_path_buf());
            Ok(())
        })
        .unwrap();
        assert!(trashed.into_inner().unwrap().is_empty());
        assert!(originals.join("IMG_0001.jpg").exists());
        assert!(!by_name(&with_open(&s, |_, i| Ok(i.clone())).unwrap(), "IMG_0001.jpg").hidden);
    }
}
