//! Préparer un modèle : le rôle de chaque texte ou photo (à remplir, fixe), les champs que la classe
//! remplit (`carnet_core::fill`), et la grille de la page. Mêmes règles que `ui/src/lib/templateRole.ts`.

use carnet_core::{Document, ElementKind, ZoneKind, fill, grid};
use serde_json::{Value, json};

/// Les champs possibles, pour les messages et la description de l'outil.
pub fn fields_hint() -> String {
    format!("{}, Lettre 1, Lettre 2… (en capitale) ou lettre 1… (telle qu'elle s'écrit)", fill::FIELD_PROMPTS.join(", "))
}

/// Une invite de champ que la classe remplit, sinon une erreur qui les liste.
pub fn check_field(field: &str) -> Result<(), String> {
    if fill::is_field_prompt(field) { Ok(()) } else { Err(format!("champ inconnu : {field} (champs : {})", fields_hint())) }
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key).and_then(Value::as_str).ok_or(format!("« {key} » manque"))
}

/// Rôle d'un texte ou d'une photo dans le modèle : `fill` (une zone, vidée à chaque usage ; `field` en fait
/// un champ de la classe), `fixed` (verrouillé, gardé tel quel) ou `none` (pas gardé dans le modèle).
pub fn set_template_role(doc: &mut Document, args: &Value) -> Result<Value, String> {
    let id = str_arg(args, "element_id")?;
    let role = str_arg(args, "role")?;
    let field = args.get("field").and_then(Value::as_str);
    if let Some(f) = field {
        check_field(f)?;
    }
    let e = doc.pages.iter_mut().flat_map(|p| &mut p.elements).find(|e| e.id == id).ok_or(format!("aucun élément « {id} »"))?;
    if !matches!(e.kind, ElementKind::Text(_) | ElementKind::Photo(_)) {
        return Err("seuls les textes et les photos ont un rôle : formes et images sont toujours gardées dans le modèle".into());
    }
    if e.locked {
        return Err(format!("l'élément « {id} » est verrouillé par l'utilisateur"));
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
                t.prompt = Some(match field {
                    Some(f) => f.to_string(),
                    None => t.prompt.clone().filter(|p| !p.is_empty()).unwrap_or(if words.is_empty() { "Texte".into() } else { words }),
                });
            } else if field.is_some() {
                return Err("un champ (« field ») se met sur un texte ; une case photo vide reçoit la photo ou le symbole de l'élève".into());
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
        set_template_role(&mut d, &json!({ "element_id": t, "role": "fill", "field": "Âge" })).unwrap();
        let ElementKind::Text(tp) = &el(&d, &t).kind else { unreachable!() };
        assert_eq!(tp.prompt.as_deref(), Some("Âge"));
        assert!(set_template_role(&mut d, &json!({ "element_id": t, "role": "fill", "field": "Taille" })).unwrap_err().contains("Prénom"));
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
    fn a_text_can_be_added_as_a_field() {
        let mut d = Document::new("x");
        let t = new_id_of(edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 50, "h_mm": 10, "field": "Prénom", "style": "prenom" })).unwrap());
        let ElementKind::Text(tp) = &el(&d, &t).kind else { unreachable!() };
        assert_eq!((el(&d, &t).zone, tp.prompt.as_deref(), tp.content.is_blank(), tp.style.named.as_deref()), (true, Some("Prénom"), true, Some("prenom")));
        assert!(edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 5, "h_mm": 5 })).unwrap_err().contains("content"));
    }

    #[test]
    fn the_view_names_known_fields_only() {
        let mut d = Document::new("x");
        let a = new_id_of(edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 50, "h_mm": 10, "field": "Lettre 2" })).unwrap());
        let b = new_id_of(edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 20, "w_mm": 50, "h_mm": 10, "content": "Chez Mamie Josette" })).unwrap());
        set_template_role(&mut d, &json!({ "element_id": b, "role": "fill" })).unwrap();
        edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 1, "y_mm": 40, "w_mm": 50, "h_mm": 10, "content": "Joyeux anniversaire {prénom}" })).unwrap();
        let v = view::layout(&d, None, &[]);
        let els = v["pages"][0]["elements"].as_array().unwrap();
        assert_eq!(els.iter().find(|e| e["id"] == a.as_str()).unwrap()["field"], "Lettre 2");
        assert!(els.iter().find(|e| e["id"] == b.as_str()).unwrap()["field"].is_null());
        assert_eq!(els[2]["class_markers"], true);
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
        let name = new_id_of(edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 15, "y_mm": 15, "w_mm": 80, "h_mm": 15, "field": "Prénom" })).unwrap());
        let age = new_id_of(edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 15, "y_mm": 32, "w_mm": 80, "h_mm": 12, "field": "Âge" })).unwrap());
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
}
