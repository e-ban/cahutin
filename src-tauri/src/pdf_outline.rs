//! Signets des chapitres dans le PDF exporté : le moteur d'impression de la vue web n'en pose pas, ils sont ajoutés au
//! fichier une fois enregistré. Le lecteur PDF les montre dans son panneau à l'ouverture.

use lopdf::{Bookmark, Document, Object};
use serde::Deserialize;
use std::path::Path;

/// Un signet : le titre du chapitre et le rang de sa première page dans le PDF (0 : la première).
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct OutlineEntry {
    pub title: String,
    pub page: usize,
}

/// Ajoute les signets au PDF `path`. Le fichier modifié est écrit à côté puis remplace l'original : en cas d'échec, le
/// PDF reste tel qu'il était, sans signets.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn add_outline(path: &Path, entries: &[OutlineEntry]) -> Result<(), String> {
    if entries.is_empty() {
        return Ok(());
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages = doc.get_pages();
    let mut added = false;
    for e in entries {
        if let Some(&page) = pages.get(&(e.page as u32 + 1)) {
            doc.add_bookmark(Bookmark::new(e.title.clone(), [0.0, 0.0, 0.0], 0, page), None);
            added = true;
        }
    }
    if !added {
        return Ok(());
    }
    let outline = doc.build_outline().ok_or("aucun signet")?;
    if let Ok(Object::Dictionary(d)) = doc.get_object_mut(outline) {
        d.set("Type", Object::Name(b"Outlines".to_vec()));
    }
    let catalog = doc.catalog_mut().map_err(|e| e.to_string())?;
    catalog.set("Outlines", Object::Reference(outline));
    catalog.set("PageMode", Object::Name(b"UseOutlines".to_vec()));
    let tmp = path.with_extension("pdf.signets");
    doc.save(&tmp).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{Dictionary, Stream, dictionary};

    /// Un PDF de `n` pages vides, comme en écrit un moteur d'impression.
    fn blank_pdf(path: &Path, n: usize) {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let kids: Vec<Object> = (0..n)
            .map(|_| {
                let content = doc.add_object(Stream::new(Dictionary::new(), Vec::new()));
                doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages_id, "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()], "Contents" => content }).into()
            })
            .collect();
        doc.objects.insert(pages_id, dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => n as i64 }.into());
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog);
        doc.save(path).unwrap();
    }

    fn titles(path: &Path) -> Vec<(String, u32)> {
        let doc = Document::load(path).unwrap();
        let pages: std::collections::HashMap<_, _> = doc.get_pages().into_iter().map(|(n, id)| (id, n)).collect();
        let root = doc.catalog().unwrap().get(b"Outlines").unwrap().as_reference().unwrap();
        let mut next = doc.get_dictionary(root).unwrap().get(b"First").ok().map(|o| o.as_reference().unwrap());
        let mut out = vec![];
        while let Some(id) = next {
            let item = doc.get_dictionary(id).unwrap();
            // Un titre avec des accents est en UTF-16 (marque FE FF), les autres tels quels.
            let raw = item.get(b"Title").unwrap().as_str().unwrap();
            let title = match raw {
                [0xFE, 0xFF, rest @ ..] => String::from_utf16(&rest.chunks(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect::<Vec<_>>()).unwrap(),
                _ => String::from_utf8(raw.to_vec()).unwrap(),
            };
            let action = doc.get_dictionary(item.get(b"A").unwrap().as_reference().unwrap()).unwrap();
            let page = pages[&action.get(b"D").unwrap().as_array().unwrap()[0].as_reference().unwrap()];
            out.push((title, page));
            next = item.get(b"Next").ok().map(|o| o.as_reference().unwrap());
        }
        out
    }

    #[test]
    fn one_bookmark_per_chapter_at_its_first_page() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cahier.pdf");
        blank_pdf(&path, 5);
        let entries = [OutlineEntry { title: "Septembre".into(), page: 1 }, OutlineEntry { title: "Décembre".into(), page: 3 }];
        add_outline(&path, &entries).unwrap();
        assert_eq!(titles(&path), [("Septembre".to_string(), 2), ("Décembre".to_string(), 4)]);
        let doc = Document::load(&path).unwrap();
        assert_eq!(doc.get_pages().len(), 5, "aucune page perdue");
        assert_eq!(doc.catalog().unwrap().get(b"PageMode").unwrap().as_name().unwrap(), b"UseOutlines");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1, "pas de fichier de travail laissé");
    }

    #[test]
    fn without_chapters_or_on_a_bad_file_the_pdf_is_left_as_it_was() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cahier.pdf");
        blank_pdf(&path, 2);
        let before = std::fs::read(&path).unwrap();
        add_outline(&path, &[]).unwrap();
        add_outline(&path, &[OutlineEntry { title: "Loin".into(), page: 9 }]).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let broken = dir.path().join("abime.pdf");
        std::fs::write(&broken, b"pas un pdf").unwrap();
        assert!(add_outline(&broken, &[OutlineEntry { title: "A".into(), page: 0 }]).is_err());
        assert_eq!(std::fs::read(&broken).unwrap(), b"pas un pdf");
    }
}
