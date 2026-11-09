//! Préparer un modèle : le rôle de chaque texte ou photo (à remplir, fixe), les champs que la classe
//! remplit (`carnet_core::fields`, entre accolades), et la grille de la page. Mêmes règles que
//! `ui/src/lib/templateRole.ts`.

use carnet_core::{Document, ElementKind, RichText, ZoneKind, fields, grid};
use serde_json::{Value, json};

/// Les champs possibles, pour les messages et la description de l'outil.
pub fn fields_hint() -> String {
    let simple: Vec<String> = fields::KEYS.iter().map(|k| format!("{{{k}}}")).collect();
    let auto = |keys: &[&str]| keys.iter().map(|k| format!("{{{k}}}")).collect::<Vec<_>>().join(", ");
    format!(
        "{}, {{PRENOM_1}}, {{PRENOM_2}}… (une lettre en capitale), {{prenom_1}}… (telle qu'elle s'écrit), {{anniversaires_mars}}… (affiche de toute la classe) ; \
la date de la page : {} ; la classe : {}",
        simple.join(", "),
        auto(&fields::DATE_KEYS),
        auto(&fields::CLASS_KEYS)
    )
}

/// Le nom d'un champ de la classe, sans accolades ni accents ; sinon une erreur qui les liste.
pub fn check_field(field: &str) -> Result<String, String> {
    let f = field.trim().trim_start_matches('{').trim_end_matches('}');
    let known = matches!(fields::split(&format!("{{{f}}}"))[..], [fields::Piece::Field(_)]);
    if known && fields::fold(f) == f {
        Ok(f.to_string())
    } else {
        Err(format!("champ inconnu : {field} (champs, sans accents : {})", fields_hint()))
    }
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key).and_then(Value::as_str).ok_or(format!("« {key} » manque"))
}

/// Rôle d'un texte ou d'une photo dans le modèle : `fill` (une zone, vidée à chaque usage), `fixed`
/// (verrouillé, gardé tel quel) ou `none` (pas gardé dans le modèle). Avec `field`, le texte devient ce champ
/// de la classe, gardé tel quel. Un texte qui contient déjà un champ n'a pas de rôle à choisir.
pub fn set_template_role(doc: &mut Document, args: &Value) -> Result<Value, String> {
    let id = str_arg(args, "element_id")?;
    let role = str_arg(args, "role")?;
    let field = args.get("field").and_then(Value::as_str).map(check_field).transpose()?;
    let e = doc.pages.iter_mut().flat_map(|p| &mut p.elements).find(|e| e.id == id).ok_or(format!("aucun élément « {id} »"))?;
    if !matches!(e.kind, ElementKind::Text(_) | ElementKind::Photo(_)) {
        return Err("seuls les textes et les photos ont un rôle : formes et images sont toujours gardées dans le modèle".into());
    }
    if e.locked {
        return Err(format!("l'élément « {id} » est verrouillé par l'utilisateur"));
    }
    if let Some(f) = field {
        let ElementKind::Text(t) = &mut e.kind else {
            return Err("un champ (« field ») se met sur un texte ; une case photo vide reçoit la photo ou le symbole de l'élève".into());
        };
        t.content = RichText::plain(&format!("{{{f}}}"));
        t.prompt = None;
        e.zone = false;
        return Ok(json!({ "ok": true }));
    }
    if matches!(&e.kind, ElementKind::Text(t) if fields::has_fields_text(t) || fields::has_class_fields_text(t) || fields::has_auto_fields_text(t)) {
        return Err("ce texte contient un champ de la classe : il est gardé tel quel dans le modèle, sans rôle à choisir".into());
    }
    match role {
        "fixed" => (e.zone, e.locked) = (false, true),
        "none" => e.zone = false,
        "fill" => {
            e.zone = true;
            // Comme `withTemplateRole` : un texte garde ses mots comme invite, sauf s'il en a déjà une.
            if let ElementKind::Text(t) = &mut e.kind {
                let words = t.content.paragraphs.iter().map(|p| p.runs.iter().map(|r| r.text.as_str()).collect::<String>()).collect::<Vec<_>>().join(" ");
                let words = words.trim().to_string();
                t.prompt = Some(t.prompt.clone().filter(|p| !p.is_empty()).unwrap_or(if words.is_empty() { "Texte".into() } else { words }));
            }
        }
        r => return Err(format!("rôle inconnu : {r} (fill, fixed, none)")),
    }
    Ok(json!({ "ok": true }))
}

/// Grille de la page (modèles faits de cases) : la poser, fusionner ou séparer des cases, changer le type
/// d'une case, ou la retirer (les cases restent, libres).
pub fn page_grid(doc: &mut Document, args: &Value) -> Result<Value, String> {
    let index = args.get("page").and_then(Value::as_u64).ok_or("« page » manque")? as usize;
    let action = str_arg(args, "action")?;
    let count = |k: &str| match args.get(k).and_then(Value::as_u64) {
        Some(n @ 1..=10) => Ok(n as u8),
        _ => Err(format!("« {k} » doit être un entier de 1 à 10")),
    };
    let page = doc.pages.get_mut(index).ok_or(format!("aucune page {index}"))?;
    let r = match action {
        "set" => {
            let (cols, rows) = (count("cols")?, count("rows")?);
            grid::set_grid(page, cols, rows);
            Ok(())
        }
        "merge" => grid::merge_cells(page, str_arg(args, "cell_id")?, str_arg(args, "other_cell_id")?),
        "split" => grid::split_cell(page, str_arg(args, "cell_id")?),
        "set_kind" => {
            let kind = match str_arg(args, "kind")? {
                "photo" => ZoneKind::Photo,
                "text" => ZoneKind::Text,
                k => return Err(format!("type inconnu : {k} (photo, text)")),
            };
            grid::set_cell_kind(page, str_arg(args, "cell_id")?, kind)
        }
        "remove" => {
            grid::liberate(page);
            Ok(())
        }
        a => return Err(format!("action inconnue : {a} (set, merge, split, set_kind, remove)")),
    };
    r.map_err(|e| e.message_fr().to_string())?;
    Ok(json!({ "ok": true }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{edit, tables, view};
    use carnet_core::{class::Student, *};
    use serde_json::json;

    fn el<'a>(d: &'a Document, id: &str) -> &'a Element {
        d.pages.iter().flat_map(|p| &p.elements).find(|e| e.id == id).unwrap()
    }

    fn new_id_of(v: Value) -> String {
        v["element_id"].as_str().unwrap().to_string()
    }

    #[test]
    fn roles_fill_fixed_and_none() {
        let mut d = Document::new("x");
        let t = new_id_of(edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 50, "h_mm": 10, "content": "Mon titre" })).unwrap());
        set_template_role(&mut d, &json!({ "element_id": t, "role": "fill" })).unwrap();
        let ElementKind::Text(tp) = &el(&d, &t).kind else { unreachable!() };
        // Comme dans l'interface : ses mots deviennent l'invite.
        assert_eq!((el(&d, &t).zone, tp.prompt.as_deref()), (true, Some("Mon titre")));
        set_template_role(&mut d, &json!({ "element_id": t, "role": "none" })).unwrap();
        assert!(!el(&d, &t).zone);
        set_template_role(&mut d, &json!({ "element_id": t, "role": "fixed" })).unwrap();
        assert!(el(&d, &t).locked && !el(&d, &t).zone);
        // Un élément verrouillé ne se déverrouille pas : ce peut être l'enseignant qui l'a fait.
        assert!(set_template_role(&mut d, &json!({ "element_id": t, "role": "none" })).unwrap_err().contains("verrouillé"));
        let s = new_id_of(edit::add_shape(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 9, "h_mm": 9, "shape": "rect" })).unwrap());
        assert!(set_template_role(&mut d, &json!({ "element_id": s, "role": "fill" })).unwrap_err().contains("toujours"));
        let p = new_id_of(edit::add_photo_zone(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 9, "h_mm": 9 })).unwrap());
        set_template_role(&mut d, &json!({ "element_id": p, "role": "fixed" })).unwrap();
        assert!(el(&d, &p).locked && !el(&d, &p).zone);
    }

    #[test]
    fn a_field_is_a_text_holding_it() {
        let mut d = Document::new("x");
        let t = new_id_of(edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 50, "h_mm": 10, "field": "prenom", "style": "prenom" })).unwrap());
        let words = |d: &Document, id: &str| match &el(d, id).kind {
            ElementKind::Text(t) => t.content.paragraphs.iter().flat_map(|p| &p.runs).map(|r| r.text.clone()).collect::<String>(),
            _ => unreachable!(),
        };
        let ElementKind::Text(tp) = &el(&d, &t).kind else { unreachable!() };
        assert_eq!((el(&d, &t).zone, tp.prompt.as_deref(), words(&d, &t).as_str(), tp.style.named.as_deref()), (false, None, "{prenom}", Some("prenom")));
        // Un texte devient un champ ; il n'a plus de rôle à choisir.
        let u = new_id_of(edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 20, "w_mm": 50, "h_mm": 10, "content": "Mon titre" })).unwrap());
        set_template_role(&mut d, &json!({ "element_id": u, "role": "fill", "field": "{PRENOM_2}" })).unwrap();
        assert_eq!((el(&d, &u).zone, words(&d, &u).as_str()), (false, "{PRENOM_2}"));
        assert!(set_template_role(&mut d, &json!({ "element_id": u, "role": "fill" })).unwrap_err().contains("champ"));
        assert!(set_template_role(&mut d, &json!({ "element_id": u, "role": "fill", "field": "taille" })).unwrap_err().contains("{prenom}"));
        assert!(set_template_role(&mut d, &json!({ "element_id": u, "role": "fill", "field": "âge" })).unwrap_err().contains("sans accents"));
        assert!(edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 5, "h_mm": 5 })).unwrap_err().contains("content"));
    }

    #[test]
    fn date_and_class_fields_are_fields_too() {
        let mut d = Document::new("x");
        for f in ["date", "annee_scolaire", "ecole", "nombre_eleves"] {
            assert_eq!(check_field(f).unwrap(), f);
            assert!(fields_hint().contains(&format!("{{{f}}}")), "{f} manque dans la liste");
        }
        let t = new_id_of(edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 50, "h_mm": 10, "content": "Le {date}" })).unwrap());
        assert!(set_template_role(&mut d, &json!({ "element_id": t, "role": "fill" })).unwrap_err().contains("champ"));
    }

    #[test]
    fn the_view_lists_the_fields_of_each_text() {
        let mut d = Document::new("x");
        let a = new_id_of(edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 50, "h_mm": 10, "field": "PRENOM_2" })).unwrap());
        let b = new_id_of(edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 20, "w_mm": 50, "h_mm": 10, "content": "Chez Mamie Josette" })).unwrap());
        set_template_role(&mut d, &json!({ "element_id": b, "role": "fill" })).unwrap();
        edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 40, "w_mm": 50, "h_mm": 10, "content": "Joyeux anniversaire {prenom}, {age} {toto}" })).unwrap();
        let v = view::layout(&d, None, &[]);
        let els = v["pages"][0]["elements"].as_array().unwrap();
        assert_eq!(els.iter().find(|e| e["id"] == a.as_str()).unwrap()["fields"], json!(["PRENOM_2"]));
        assert_eq!(els.iter().find(|e| e["id"] == b.as_str()).unwrap()["fields"], json!([]));
        assert_eq!(els[2]["fields"], json!(["prenom", "age"]));
        assert!(!v.to_string().contains("Josette"), "une invite libre ne sort pas");
        assert!(!v.to_string().contains("anniversaire {"), "ni un texte");
    }

    #[test]
    fn page_grid_actions() {
        let mut d = Document::new("x");
        page_grid(&mut d, &json!({ "page": 0, "action": "set", "cols": 2, "rows": 3 })).unwrap();
        let cells: Vec<String> = d.pages[0].grid.as_ref().unwrap().cells.iter().map(|c| c.element_id.clone()).collect();
        assert_eq!(cells.len(), 6);
        page_grid(&mut d, &json!({ "page": 0, "action": "merge", "cell_id": cells[0], "other_cell_id": cells[1] })).unwrap();
        assert_eq!(d.pages[0].grid.as_ref().unwrap().cells.len(), 5);
        page_grid(&mut d, &json!({ "page": 0, "action": "split", "cell_id": cells[0] })).unwrap();
        page_grid(&mut d, &json!({ "page": 0, "action": "set_kind", "cell_id": cells[2], "kind": "text" })).unwrap();
        assert!(matches!(el(&d, &cells[2]).kind, ElementKind::Text(_)));
        let err = page_grid(&mut d, &json!({ "page": 0, "action": "split", "cell_id": cells[0] })).unwrap_err();
        assert_eq!(err, grid::GridError::NotSpanned.message_fr());
        page_grid(&mut d, &json!({ "page": 0, "action": "remove" })).unwrap();
        assert!(d.pages[0].grid.is_none() && d.pages[0].elements.len() == 6);
        assert!(page_grid(&mut d, &json!({ "page": 0, "action": "set", "cols": 0, "rows": 3 })).is_err());
    }

    /// Ce qu'on dit au modèle doit marcher : une étiquette faite avec les outils, répétée, devient une
    /// planche que Cahutin remplit avec la classe, un prénom par étiquette.
    #[test]
    fn a_label_made_with_the_tools_fills_with_the_class() {
        let mut d = Document::new("x");
        let frame = new_id_of(edit::add_shape(&mut d, &json!({ "page": 0, "x_mm": 10, "y_mm": 10, "w_mm": 90, "h_mm": 40, "shape": "rect", "border_color": "#333333" })).unwrap());
        let name = new_id_of(edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 15, "y_mm": 15, "w_mm": 80, "h_mm": 15, "field": "prenom" })).unwrap());
        let age = new_id_of(edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 15, "y_mm": 32, "w_mm": 80, "h_mm": 12, "field": "age" })).unwrap());
        tables::repeat(&mut d, &json!({ "element_ids": [frame, name, age], "cols": 2, "rows": 2 })).unwrap();
        let page = &d.pages[0];
        assert_eq!(fill::suggested_mode(page), fill::FillMode::Sheet);
        assert_eq!(fill::block_count(page), 4);
        // La vue le dit au modèle, pour qu'il vérifie son travail.
        assert_eq!(view::layout(&d, None, &[])["pages"][0]["class_fill"], json!({ "mode": "sheet", "blocks": 4 }));
        let class = [Student::new("Léa", "Martin"), Student::new("Noé", "Petit")];
        let out = fill::generate(page, &class, fill::FillMode::Sheet);
        let text = serde_json::to_string(&out).unwrap();
        assert!(text.contains("Léa") && text.contains("Noé"), "{text}");
    }

    /// Le même chemin que l'application : la planche enregistrée comme modèle (en gardant les textes de
    /// l'assistant et leurs copies, comme `writtenWithCopies` dans l'interface), puis remplie avec la classe.
    #[test]
    fn a_saved_sheet_template_keeps_its_texts_and_cut_marks_and_fills() {
        let mut d = Document::new("x");
        let frame = new_id_of(edit::add_shape(&mut d, &json!({ "page": 0, "x_mm": 10, "y_mm": 10, "w_mm": 90, "h_mm": 40, "shape": "rect" })).unwrap());
        let name = new_id_of(edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 15, "y_mm": 15, "w_mm": 80, "h_mm": 15, "field": "prenom" })).unwrap());
        let wish = new_id_of(edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 15, "y_mm": 32, "w_mm": 80, "h_mm": 12, "content": "Bon anniversaire {prenom} !" })).unwrap());
        crate::arrange::group(&mut d, &json!({ "element_ids": [frame, name, wish] })).unwrap();
        crate::decor::cut_marks(&mut d, &json!({ "element_ids": [frame], "style": "corners" })).unwrap();
        tables::repeat(&mut d, &json!({ "element_ids": [frame], "cols": 2, "rows": 2 })).unwrap();
        let page = &d.pages[0];
        let keep: Vec<String> = page.elements.iter().filter(|e| matches!(&e.kind, ElementKind::Text(t) if !t.content.is_blank())).map(|e| e.id.clone()).collect();
        // Les quatre champs {prenom} et les quatre vœux écrits par l'assistant.
        assert_eq!(keep.len(), 8);
        let saved = templates::from_page_keeping("Anniversaires", page, &photo::AssetStore::default(), &keep);
        assert_eq!(saved.page.elements.iter().filter(|e| crate::decor::is_cut_mark(e)).count(), 32);
        assert_eq!(fill::suggested_mode(&saved.page), fill::FillMode::Sheet);
        let class = [Student::new("Léa", "Martin"), Student::new("Noé", "Petit")];
        let text = serde_json::to_string(&fill::generate(&saved.page, &class, fill::FillMode::Sheet)).unwrap();
        assert!(text.contains("Bon anniversaire Léa !") && text.contains("Bon anniversaire Noé !"), "{text}");
    }
}
