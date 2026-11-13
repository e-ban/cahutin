//! Migration : retrouver, parmi les photos des autres dossiers suivis, l'original de chaque copie réduite
//! faite par l'ancienne photothèque. Les empreintes (dHash) se calculent sur les vignettes du cache, jamais
//! sur les originaux en pleine taille. Ces fonctions ne touchent aucun fichier de l'enseignant : seul le
//! cache de vignettes peut s'écrire.

use super::thumbs::thumb;
use super::{ClassPhoto, Phototheque};
use crate::model::Id;
use image::{DynamicImage, imageops::FilterType};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use ts_rs::TS;

/// Distance de Hamming la plus grande entre deux empreintes pour parler de la même photo.
const MAX_DISTANCE: u32 = 10;
/// La même, pour un original sans date d'appareil pris un autre jour d'après son fichier. Mesuré sur des
/// photos générées : une copie est à 0 ou 1 de son original, deux photos différentes à 6 ou plus. Deux photos d'une
/// rafale sont à 2 : ce qui les départage est la règle du seul candidat le plus proche, pas ce seuil.
const MAX_DISTANCE_UNDATED: u32 = 4;
/// Écart de proportions toléré (largeur sur hauteur), en proportion.
const MAX_RATIO_GAP: f64 = 0.01;

/// Empreinte de 64 bits : l'image réduite à 9 x 8 en gris, un bit par comparaison de deux voisins.
pub fn dhash(img: &DynamicImage) -> u64 {
    let small = img.resize_exact(9, 8, FilterType::Triangle).to_luma8();
    let mut hash = 0u64;
    for y in 0..8 {
        for x in 0..8 {
            hash = (hash << 1) | u64::from(small.get_pixel(x, y).0[0] > small.get_pixel(x + 1, y).0[0]);
        }
    }
    hash
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
pub struct Match {
    pub copy: Id,
    pub original: Id,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq, TS)]
#[ts(export)]
pub struct OriginalsReport {
    pub matches: Vec<Match>,
    pub unmatched: Vec<Id>,
}

/// Empreinte d'une photo, lue sur sa vignette du cache (jamais sur le fichier en pleine taille). Une photo
/// abîmée qui fait paniquer le décodeur n'a pas d'empreinte, comme une photo illisible.
fn hash_of(library_dir: &Path, p: &ClassPhoto) -> Option<u64> {
    std::panic::catch_unwind(|| {
        let img = image::open(thumb(library_dir, p).ok()?).ok()?;
        Some(dhash(&img))
    })
    .ok()
    .flatten()
}

/// Une copie faite par l'ancienne photothèque : dans le dossier des copies, sous le nom `<id>.<ext>` de sa photo
/// (la règle de l'interface). Toute autre photo de ce dossier est à l'enseignant : jamais une copie.
pub fn is_copy(p: &ClassPhoto, copies_folder: &Path) -> bool {
    let path = Path::new(&p.path);
    path.starts_with(copies_folder) && path.extension().is_some() && path.file_stem().is_some_and(|s| *s == *p.id)
}

fn ratio(p: &ClassPhoto) -> f64 {
    f64::from(p.width_px) / f64::from(p.height_px.max(1))
}

/// Proportions de la photo telle qu'elle se montre : les dimensions de l'index sont celles du fichier, sans
/// l'étiquette d'orientation ; une étiquette 5 à 8 (photo de téléphone tenu en portrait) les échange. Seul
/// l'en-tête du fichier est lu.
fn shown_ratio(p: &ClassPhoto) -> f64 {
    let turned = std::fs::File::open(&p.path)
        .ok()
        .and_then(|f| exif::Reader::new().read_from_container(&mut std::io::BufReader::new(f)).ok())
        .and_then(|e| e.get_field(exif::Tag::Orientation, exif::In::PRIMARY).and_then(|f| f.value.get_uint(0)))
        .is_some_and(|o| (5..=8).contains(&o));
    if turned { 1.0 / ratio(p) } else { ratio(p) }
}

/// L'original le plus proche de la copie parmi les candidats : proportions à moins de 1 %, dHash à au plus 10,
/// même jour si les deux sont datées. Un seul candidat au plus proche, sinon on ne devine pas.
/// Avec `undated`, un candidat d'un autre jour est admis s'il n'a pas de date d'appareil (sa date est alors
/// celle du fichier, qui change quand le dossier est recopié) et s'il ressemble de plus près encore.
fn closest(copy: &ClassPhoto, h: u64, candidates: &[(&ClassPhoto, u64)], undated: bool) -> Option<Id> {
    let mut best: Vec<(&ClassPhoto, u32)> = vec![];
    for &(c, ch) in candidates {
        let same_day = match (&copy.taken, &c.taken) {
            (Some(a), Some(b)) => a == b,
            _ => true,
        };
        let distance = (h ^ ch).count_ones();
        if distance > MAX_DISTANCE || (!same_day && !(undated && distance <= MAX_DISTANCE_UNDATED)) {
            continue;
        }
        // Les copies sont déjà redressées ; le fichier de l'original est lu seulement s'il est assez proche.
        if (ratio(copy) / shown_ratio(c) - 1.0).abs() >= MAX_RATIO_GAP || (!same_day && super::capture_date_of_file(Path::new(&c.path)).is_some()) {
            continue;
        }
        match best.first() {
            Some(&(_, d)) if distance > d => {}
            Some(&(_, d)) if distance == d => best.push((c, distance)),
            _ => best = vec![(c, distance)],
        }
    }
    if let [(c, _)] = best[..] { Some(c.id.clone()) } else { None }
}

/// Pour chaque copie du dossier des copies (`copies_folder`, voir `is_copy`), l'original parmi les photos des autres dossiers
/// suivis (voir `closest`). Une copie sans original le même jour est cherchée une seconde fois parmi les photos
/// sans date d'appareil.
/// Deux copies du même original le retrouvent toutes les deux : leurs étiquettes seront réunies sur lui.
/// `progress(regardées, à regarder)` est appelée après chaque photo ; si elle rend `false`, la recherche
/// s'arrête et ne rend rien (les vignettes déjà faites restent dans le cache).
pub fn find_originals(lib: &Phototheque, library_dir: &Path, copies_folder: &str, progress: &mut dyn FnMut(usize, usize) -> bool) -> Option<OriginalsReport> {
    let root = Path::new(copies_folder);
    let in_copies = |p: &ClassPhoto| Path::new(&p.path).starts_with(root);
    // Les copies déjà masquées ou absentes ne sont plus à traiter.
    let copies: Vec<&ClassPhoto> = lib.photos.iter().filter(|p| is_copy(p, root) && !p.hidden && !p.missing).collect();
    let others: Vec<&ClassPhoto> = lib.photos.iter().filter(|p| !in_copies(p) && !p.hidden && !p.missing).collect();
    let total = copies.len() + others.len();
    let mut seen = 0;
    let mut hashed = |p: &ClassPhoto| {
        let h = hash_of(library_dir, p);
        seen += 1;
        progress(seen, total).then_some(h)
    };
    let mut candidates = Vec::with_capacity(others.len());
    for p in others {
        if let Some(h) = hashed(p)? {
            candidates.push((p, h));
        }
    }
    let mut report = OriginalsReport::default();
    for copy in copies {
        let found = hashed(copy)?.and_then(|h| closest(copy, h, &candidates, false).or_else(|| closest(copy, h, &candidates, true)));
        match found {
            Some(original) => report.matches.push(Match { copy: copy.id.clone(), original }),
            None => report.unmatched.push(copy.id.clone()),
        }
    }
    Some(report)
}

/// Reporte élèves et activités de chaque copie sur son original (réunis, sans doublon) et masque la copie ;
/// rend les chemins des copies à mettre à la corbeille.
pub fn apply_originals(lib: &mut Phototheque, matches: &[Match]) -> Vec<PathBuf> {
    let mut trash = vec![];
    for m in matches {
        let Some(copy) = lib.photos.iter().find(|p| p.id == m.copy).cloned() else { continue };
        let Some(original) = lib.photos.iter_mut().find(|p| p.id == m.original) else { continue };
        for s in &copy.students {
            if !original.students.contains(s) {
                original.students.push(s.clone());
            }
        }
        for t in &copy.tags {
            if !original.tags.contains(t) {
                original.tags.push(t.clone());
            }
        }
        if let Some(c) = lib.photos.iter_mut().find(|p| p.id == m.copy) {
            c.hidden = true;
        }
        trash.push(PathBuf::from(copy.path));
    }
    trash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phototheque::{FORMAT_VERSION, follow, scan};
    use image::{ImageFormat, Rgb, RgbImage};
    use std::io::Cursor;

    /// Une photo de 1200 x 900 dont les bandes verticales claires et sombres dépendent de `seed`.
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

    fn jpeg(img: &RgbImage) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, ImageFormat::Jpeg).unwrap();
        out.into_inner()
    }

    struct Setup {
        _tmp: tempfile::TempDir,
        library: PathBuf,
        originals: PathBuf,
        lib: Phototheque,
    }

    /// Les copies (`copy_seeds`, réduites comme l'ancienne photothèque) et les originaux (`original_seeds`).
    fn setup(copy_seeds: &[u32], original_seeds: &[u32]) -> Setup {
        let tmp = tempfile::tempdir().unwrap();
        let library = tmp.path().join("Classe");
        let originals = tmp.path().join("Originaux");
        std::fs::create_dir_all(&library).unwrap();
        std::fs::create_dir_all(&originals).unwrap();
        for s in copy_seeds {
            let asset = crate::photo::import_bytes(&jpeg(&pattern(*s))).unwrap();
            std::fs::write(library.join(format!("copie{s}.jpg")), &*asset.bytes).unwrap();
        }
        for (i, s) in original_seeds.iter().enumerate() {
            std::fs::write(originals.join(format!("o{i}-{s}.jpg")), jpeg(&pattern(*s))).unwrap();
        }
        let mut lib = Phototheque { format_version: FORMAT_VERSION, folders: vec![], photos: vec![], migration_settled: false };
        follow(&mut lib, &library).unwrap();
        follow(&mut lib, &originals).unwrap();
        scan(&mut lib, &library, &mut |_| {});
        // Comme l'ancienne photothèque : une copie porte pour nom l'identifiant de sa photo.
        for p in lib.photos.iter_mut().filter(|p| Path::new(&p.path).starts_with(&library)) {
            p.id = Path::new(&p.path).file_stem().unwrap().to_string_lossy().into_owned();
        }
        Setup { _tmp: tmp, library, originals, lib }
    }

    fn id_of(s: &Setup, name: &str) -> Id {
        s.lib.photos.iter().find(|p| p.path.ends_with(name)).unwrap().id.clone()
    }

    fn report(s: &Setup) -> OriginalsReport {
        find_originals(&s.lib, &s.library, &s.library.to_string_lossy(), &mut |_, _| true).unwrap()
    }

    #[test]
    fn dhash_tells_patterns_apart() {
        let a = dhash(&DynamicImage::ImageRgb8(pattern(1)));
        let b = dhash(&DynamicImage::ImageRgb8(pattern(2)));
        assert_eq!(a, dhash(&DynamicImage::ImageRgb8(pattern(1))));
        assert!((a ^ b).count_ones() > 10);
    }

    #[test]
    fn three_copies_find_their_three_originals() {
        let s = setup(&[1, 2, 3], &[3, 1, 2]);
        let r = report(&s);
        assert!(r.unmatched.is_empty(), "{r:?}");
        assert_eq!(r.matches.len(), 3);
        for seed in [1, 2, 3] {
            let copy = id_of(&s, &format!("copie{seed}.jpg"));
            let m = r.matches.iter().find(|m| m.copy == copy).unwrap();
            assert_eq!(m.original, id_of(&s, &format!("-{seed}.jpg")));
        }
    }

    #[test]
    fn a_copy_without_original_is_unmatched() {
        let s = setup(&[1, 2], &[1]);
        let r = report(&s);
        assert_eq!(r.matches.len(), 1);
        assert_eq!(r.unmatched, vec![id_of(&s, "copie2.jpg")]);
    }

    #[test]
    fn two_identical_originals_make_no_match() {
        let s = setup(&[1], &[1, 1]);
        let r = report(&s);
        assert!(r.matches.is_empty());
        assert_eq!(r.unmatched, vec![id_of(&s, "copie1.jpg")]);
    }

    /// Un JPEG qui porte ces métadonnées EXIF (un bloc TIFF).
    fn jpeg_with_exif(img: &RgbImage, tiff: Vec<u8>) -> Vec<u8> {
        use image::{ExtendedColorType, ImageEncoder, codecs::jpeg::JpegEncoder};
        let mut out = Vec::new();
        let mut encoder = JpegEncoder::new(&mut out);
        encoder.set_exif_metadata(tiff).unwrap();
        encoder.write_image(img.as_raw(), img.width(), img.height(), ExtendedColorType::Rgb8).unwrap();
        out
    }

    /// Une photo datée par l'appareil (EXIF « DateTime »), `day` écrit « 2026:10:01 ».
    fn jpeg_dated(img: &RgbImage, day: &str) -> Vec<u8> {
        let mut tiff = b"II*\0\x08\0\0\0\x01\0".to_vec();
        tiff.extend_from_slice(&[0x32, 0x01, 2, 0, 20, 0, 0, 0, 26, 0, 0, 0, 0, 0, 0, 0]);
        tiff.extend_from_slice(format!("{day} 10:00:00\0").as_bytes());
        jpeg_with_exif(img, tiff)
    }

    /// La copie est du 2 octobre, l'original (seul candidat) du 1er d'après l'index.
    fn other_days(s: &mut Setup) -> Id {
        let original = id_of(s, "-1.jpg");
        s.lib.photos.iter_mut().for_each(|p| p.taken = Some(if p.id == original { "2026-10-01".into() } else { "2026-10-02".into() }));
        original
    }

    #[test]
    fn an_original_dated_another_day_by_the_camera_is_excluded() {
        let mut s = setup(&[1], &[1]);
        std::fs::write(s.originals.join("o0-1.jpg"), jpeg_dated(&pattern(1), "2026:10:01")).unwrap();
        other_days(&mut s);
        assert!(report(&s).matches.is_empty());
        s.lib.photos.iter_mut().for_each(|p| p.taken = None);
        assert_eq!(report(&s).matches.len(), 1, "sans date dans l'index, le jour ne compte pas");
    }

    #[test]
    fn an_original_without_camera_date_is_found_whatever_the_day_of_its_file() {
        // Le dossier des originaux a été recopié : le fichier a changé de date, la photo est la même.
        let mut s = setup(&[1], &[1]);
        other_days(&mut s);
        assert_eq!(report(&s).matches.len(), 1);
    }

    /// `pattern(1)`, un morceau de bande éclairci : une photo qui ressemble, sans être la même.
    fn look_alike() -> RgbImage {
        let mut img = pattern(1);
        for (x, y, px) in img.enumerate_pixels_mut() {
            if (300..400).contains(&x) && y < 340 {
                px.0 = [255, 255, 255];
            }
        }
        img
    }

    #[test]
    fn a_look_alike_without_camera_date_from_another_day_is_not_taken_for_the_original() {
        let mut s = setup(&[1], &[1]);
        std::fs::write(s.originals.join("o0-1.jpg"), jpeg(&look_alike())).unwrap();
        let (c, o) = (dhash(&DynamicImage::ImageRgb8(pattern(1))), dhash(&DynamicImage::ImageRgb8(look_alike())));
        let d = (c ^ o).count_ones();
        assert!(d > MAX_DISTANCE_UNDATED && d <= MAX_DISTANCE, "la ressemblance du test : {d}");
        other_days(&mut s);
        assert!(report(&s).matches.is_empty(), "un autre jour : il faut ressembler de plus près");
        s.lib.photos.iter_mut().for_each(|p| p.taken = None);
        assert_eq!(report(&s).matches.len(), 1, "le même jour, cette ressemblance suffit");
    }

    #[test]
    fn other_proportions_exclude_a_candidate() {
        let mut s = setup(&[1], &[1]);
        let original = id_of(&s, "-1.jpg");
        s.lib.photos.iter_mut().find(|p| p.id == original).unwrap().width_px = 900;
        assert!(report(&s).matches.is_empty());
    }

    #[test]
    fn the_search_tells_its_progress_and_can_be_stopped() {
        let s = setup(&[1, 2], &[1, 2, 3]);
        let mut steps = vec![];
        let folder = s.library.to_string_lossy();
        let all = find_originals(&s.lib, &s.library, &folder, &mut |seen, total| {
            steps.push((seen, total));
            true
        });
        assert!(all.is_some());
        assert_eq!(steps, (1..=5).map(|i| (i, 5)).collect::<Vec<_>>());
        let mut calls = 0;
        let stopped = find_originals(&s.lib, &s.library, &folder, &mut |seen, _| {
            calls += 1;
            seen < 2
        });
        assert_eq!((stopped, calls), (None, 2), "arrêtée : pas de résultat");
    }

    /// Une photo de téléphone tenu en portrait : enregistrée en 1200 x 900, avec l'étiquette EXIF « tourner de
    /// 90° » (orientation 6) ; montrée en 900 x 1200.
    fn jpeg_turned(img: &RgbImage) -> Vec<u8> {
        let mut tiff = b"II*\0\x08\0\0\0\x01\0".to_vec();
        tiff.extend_from_slice(&[0x12, 0x01, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0]);
        jpeg_with_exif(img, tiff)
    }

    #[test]
    fn a_turned_phone_photo_finds_its_original() {
        let tmp = tempfile::tempdir().unwrap();
        let (library, originals) = (tmp.path().join("Classe"), tmp.path().join("Originaux"));
        std::fs::create_dir_all(&library).unwrap();
        std::fs::create_dir_all(&originals).unwrap();
        let raw = jpeg_turned(&pattern(1));
        let copy = crate::photo::import_bytes(&raw).unwrap();
        assert!(copy.meta.height_px > copy.meta.width_px, "la copie est redressée, en portrait");
        std::fs::write(library.join("copie.jpg"), &*copy.bytes).unwrap();
        std::fs::write(originals.join("IMG_0001.jpg"), &raw).unwrap();
        let mut lib = Phototheque { format_version: FORMAT_VERSION, folders: vec![], photos: vec![], migration_settled: false };
        follow(&mut lib, &library).unwrap();
        follow(&mut lib, &originals).unwrap();
        scan(&mut lib, &library, &mut |_| {});
        let c = lib.photos.iter_mut().find(|p| p.path.ends_with("copie.jpg")).unwrap();
        c.id = "copie".into();
        let r = find_originals(&lib, &library, &library.to_string_lossy(), &mut |_, _| true).unwrap();
        assert_eq!((r.matches.len(), r.unmatched.len()), (1, 0), "{r:?}");
    }

    #[test]
    fn a_teacher_photo_in_the_class_folder_is_not_a_copy() {
        let mut s = setup(&[1], &[1, 2]);
        // L'enseignant a rangé là une photo à lui, dont le double est dans « Originaux ».
        std::fs::write(s.library.join("IMG_0002.jpg"), jpeg(&pattern(2))).unwrap();
        scan(&mut s.lib, &s.library, &mut |_| {});
        let teacher = id_of(&s, "IMG_0002.jpg");
        let r = report(&s);
        assert_eq!(r.matches.len(), 1);
        assert!(r.matches.iter().all(|m| m.copy != teacher) && !r.unmatched.contains(&teacher), "{r:?}");
    }

    #[test]
    fn hidden_candidates_are_ignored() {
        let mut s = setup(&[1], &[1]);
        let original = id_of(&s, "-1.jpg");
        s.lib.photos.iter_mut().find(|p| p.id == original).unwrap().hidden = true;
        assert!(report(&s).matches.is_empty());
    }

    #[test]
    fn applying_merges_tags_hides_the_copy_and_returns_its_path() {
        let mut s = setup(&[1, 2], &[1]);
        let (c1, o1) = (id_of(&s, "copie1.jpg"), id_of(&s, "-1.jpg"));
        let c2 = id_of(&s, "copie2.jpg");
        for p in &mut s.lib.photos {
            if p.id == c1 {
                p.students = vec!["lea".into(), "noe".into()];
                p.tags = vec!["Ferme".into()];
            }
            if p.id == c2 {
                p.tags = vec!["Ferme".into(), "Cirque".into()];
            }
            if p.id == o1 {
                p.students = vec!["noe".into()];
                p.tags = vec!["Motricité".into(), "Ferme".into()];
            }
        }
        // Deux copies du même original : leurs étiquettes sont réunies sur lui, sans doublon.
        let both = vec![Match { copy: c1.clone(), original: o1.clone() }, Match { copy: c2.clone(), original: o1.clone() }];
        let count = |s: &Setup| std::fs::read_dir(&s.library).unwrap().count() + std::fs::read_dir(&s.originals).unwrap().count();
        let before = count(&s);
        let trash = apply_originals(&mut s.lib, &both);
        assert_eq!(before, count(&s), "rien n'est touché sur le disque");
        let o = s.lib.photos.iter().find(|p| p.id == o1).unwrap();
        assert_eq!(o.students, ["noe", "lea"]);
        assert_eq!(o.tags, ["Motricité", "Ferme", "Cirque"]);
        assert!(!o.hidden);
        assert!(s.lib.photos.iter().filter(|p| p.id == c1 || p.id == c2).all(|p| p.hidden));
        assert_eq!(trash, vec![s.library.join("copie1.jpg"), s.library.join("copie2.jpg")]);
    }
}
