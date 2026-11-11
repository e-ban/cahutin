//! Les outils de l'assistant : leur description pour le modèle, et leur application à un document en
//! mémoire. Partagés par le serveur MCP et par l'application.

use crate::{arrange, decor, edit, exercises, tables, template, view};
use carnet_core::{Document, Id};
use serde_json::{Value, json};

/// Outils qui modifient le document (chacun fait une étape de la proposition).
pub const MUTATING: &[&str] = &[
    "set_frame",
    "set_text_style",
    "set_look",
    "set_background",
    "add_text",
    "set_text",
    "add_shape",
    "add_image",
    "add_photo_zone",
    "delete_element",
    "add_page",
    "arrange",
    "group",
    "ungroup",
    "set_page_format",
    "add_table",
    "table_edit",
    "repeat",
    "set_template_role",
    "page_grid",
    "cut_marks",
    "add_caption",
    "decor",
    "exercise",
];

/// Outil traité par la boucle de dialogue, pas par `apply` : il n'agit qu'à l'acceptation.
pub const SAVE_AS_TEMPLATE: &str = "save_as_template";

pub fn apply(doc: &mut Document, name: &str, args: &Value, selection: &[Id]) -> Result<Value, String> {
    match name {
        "get_layout" => Ok(view::layout(doc, args.get("page").and_then(Value::as_u64).map(|p| p as usize), selection)),
        "set_frame" => edit::set_frame(doc, args),
        "set_text_style" => edit::set_text_style(doc, args),
        "set_look" => edit::set_look(doc, args),
        "set_background" => edit::set_background(doc, args),
        "add_text" => edit::add_text(doc, args),
        "set_text" => edit::set_text(doc, args),
        "add_shape" => edit::add_shape(doc, args),
        "add_image" => edit::add_image(doc, args),
        "add_photo_zone" => edit::add_photo_zone(doc, args),
        "delete_element" => edit::delete_element(doc, args),
        "add_page" => edit::add_page(doc, args),
        "arrange" => arrange::arrange(doc, args),
        "group" => arrange::group(doc, args),
        "ungroup" => arrange::ungroup(doc, args),
        "set_page_format" => arrange::set_page_format(doc, args),
        "add_table" => tables::add_table(doc, args),
        "table_edit" => tables::table_edit(doc, args),
        "repeat" => tables::repeat(doc, args),
        "set_template_role" => template::set_template_role(doc, args),
        "page_grid" => template::page_grid(doc, args),
        "cut_marks" => decor::cut_marks(doc, args),
        "add_caption" => decor::add_caption(doc, args),
        "decor" => decor::decor(doc, args),
        "exercise" => exercises::exercise(doc, args),
        other => Err(format!("outil inconnu : {other}")),
    }
}

/// Description des outils (format MCP : `name`, `description`, `inputSchema`). Elle part avec chaque
/// demande : courte, sans redire ce que `server::INSTRUCTIONS` pose une fois (identifiants, pages, mm,
/// couleurs, champs de la classe).
pub fn definitions() -> Value {
    let id = json!({ "type": "string" });
    let ids = json!({ "type": "array", "items": { "type": "string" } });
    let num = json!({ "type": "number" });
    let int = json!({ "type": "integer" });
    let text = json!({ "type": "string" });
    let boolean = json!({ "type": "boolean" });
    let one_of = |v: &[&str]| json!({ "type": "string", "enum": v });
    let obj = |required: &[&str], props: Value| json!({ "type": "object", "required": required, "properties": props });
    // Un nouvel élément : sa page et son cadre, puis ses propriétés.
    let frame = |extra: Value, required: &[&str]| {
        let mut props = json!({ "page": int, "x_mm": num, "y_mm": num, "w_mm": num, "h_mm": num });
        props.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        let mut req = vec!["page", "x_mm", "y_mm", "w_mm", "h_mm"];
        req.extend_from_slice(required);
        obj(&req, props)
    };
    let align = one_of(&["left", "center", "right", "justify"]);
    let valign = one_of(&["top", "middle", "bottom"]);
    json!([
        {
            "name": "get_layout",
            "description": "Mise en page : pages, puis chaque élément avec son cadre, son style et s'il est sélectionné. Ni texte ni photo. Un champ absent vaut false, aucun ou 0.",
            "inputSchema": { "type": "object", "properties": { "page": { "type": "integer", "description": "absent : toutes" } } },
        },
        {
            "name": "set_frame",
            "description": "Déplace, redimensionne ou tourne un élément ; seules les valeurs données changent.",
            "inputSchema": obj(&["element_id"], json!({ "element_id": id, "x_mm": num, "y_mm": num, "w_mm": num, "h_mm": num, "rotation_deg": num })),
        },
        {
            "name": "set_text_style",
            "description": "Allure d'un texte (pas son contenu). style : un des text_styles ; outline : lettres creuses ; writing_lines : lignes d'écriture.",
            "inputSchema": obj(&["element_id"], json!({
                "element_id": id, "font": text, "size_pt": num, "color": text, "align": align, "valign": valign, "style": text,
                "outline": boolean, "writing_lines": one_of(&["seyes", "small_squares", "lines", "dots", "none"]),
            })),
        },
        {
            "name": "set_look",
            "description": "Remplissage (none : aucun), bord, coins et ombre d'une forme, d'une case photo ou du cadre d'un texte.",
            "inputSchema": obj(&["element_id"], json!({
                "element_id": id, "fill": text, "border_color": text, "border_width_mm": num,
                "border_style": one_of(&["solid", "dashed", "dotted", "none"]), "corner_radius_mm": num, "shadow": boolean,
            })),
        },
        {
            "name": "set_background",
            "description": "Fond d'une page : couleur et motif ; seules les valeurs données changent.",
            "inputSchema": obj(&["page"], json!({ "page": int, "color": text, "pattern": { "type": ["string", "null"], "enum": ["seyes", "small_squares", "lines", "dots", null] } })),
        },
        {
            "name": "add_text",
            "description": "Ajoute un texte ; field au lieu de content : zone remplie avec la classe.",
            "inputSchema": frame(json!({ "content": text, "field": text, "style": text, "font": text, "size_pt": num, "color": text, "align": align }), &["content"]),
        },
        {
            "name": "set_text",
            "description": "Remplace le texte d'un élément (vous ne lisez pas l'ancien).",
            "inputSchema": obj(&["element_id", "content"], json!({ "element_id": id, "content": text })),
        },
        {
            "name": "add_shape",
            "description": "Ajoute une forme.",
            "inputSchema": frame(json!({ "shape": one_of(&["rect", "ellipse", "line", "arrow", "star", "heart", "bubble", "cloud"]), "fill": text, "border_color": text, "border_width_mm": num }), &["shape"]),
        },
        {
            "name": "add_image",
            "description": "Ajoute une image de la banque : query, mot-clé français (« tulipe ») ; to_color : contour à colorier.",
            "inputSchema": frame(json!({ "query": text, "to_color": boolean }), &["query"]),
        },
        {
            "name": "add_photo_zone",
            "description": "Ajoute une case photo vide.",
            "inputSchema": frame(json!({}), &[]),
        },
        {
            "name": "delete_element",
            "description": "Supprime un élément.",
            "inputSchema": obj(&["element_id"], json!({ "element_id": id })),
        },
        {
            "name": "add_page",
            "description": "Ajoute une page vide à la fin ; rend son indice.",
            "inputSchema": { "type": "object", "properties": {} },
        },
        {
            "name": "arrange",
            "description": "Met un élément et son groupe devant, derrière, en double (rend les id) ou en miroir.",
            "inputSchema": obj(&["element_id", "action"], json!({ "element_id": id, "action": one_of(&["front", "back", "duplicate", "mirror_h", "mirror_v"]) })),
        },
        {
            "name": "group",
            "description": "Groupe des éléments d'une même page.",
            "inputSchema": obj(&["element_ids"], json!({ "element_ids": ids })),
        },
        {
            "name": "ungroup",
            "description": "Dissocie le groupe d'un élément (un niveau).",
            "inputSchema": obj(&["element_id"], json!({ "element_id": id })),
        },
        {
            "name": "set_page_format",
            "description": "Format (le contenu suit) ou orientation d'une page.",
            "inputSchema": obj(&["page"], json!({ "page": int, "format": one_of(&["A5", "A4", "A3"]), "orientation": one_of(&["portrait", "landscape"]) })),
        },
        {
            "name": "add_table",
            "description": "Ajoute un tableau de cases texte vides (1 à 50 × 50) ; rend ses cases (element_id, col, row), à remplir par set_text.",
            "inputSchema": frame(json!({ "cols": int, "rows": int }), &["cols", "rows"]),
        },
        {
            "name": "table_edit",
            "description": "Modifie un tableau depuis une case (cells) : lignes, colonnes, merge (other_cell_id), split, set_content (content, query).",
            "inputSchema": obj(&["cell_id", "action"], json!({
                "cell_id": id,
                "action": one_of(&["insert_row_above", "insert_row_below", "insert_col_left", "insert_col_right", "remove_row", "remove_col", "merge", "split", "set_content"]),
                "other_cell_id": id, "content": one_of(&["text", "image", "photo"]), "query": text,
            })),
        },
        {
            "name": "repeat",
            "description": "Répète un bloc en grille sur sa page (planche d'étiquettes) : cols × rows (1 à 20) exemplaires, lui compris, ou fill_page ; gap_mm : 5. Un groupe par exemplaire.",
            "inputSchema": obj(&["element_ids"], json!({ "element_ids": ids, "cols": int, "rows": int, "fill_page": boolean, "gap_mm": num })),
        },
        {
            "name": "set_template_role",
            "description": "Rôle dans un modèle : fill (vidé à chaque usage), fixed (gardé, verrouillé : en dernier), none (pas gardé) ; field : devient ce champ.",
            "inputSchema": obj(&["element_id", "role"], json!({ "element_id": id, "role": one_of(&["fill", "fixed", "none"]), "field": text })),
        },
        {
            "name": "page_grid",
            "description": "Grille de cases d'un modèle : set (cols × rows, 1 à 10), merge, split, set_kind (kind), remove (garde les cases).",
            "inputSchema": obj(&["page", "action"], json!({
                "page": int, "action": one_of(&["set", "merge", "split", "set_kind", "remove"]),
                "cols": int, "rows": int, "cell_id": id, "other_cell_id": id, "kind": one_of(&["photo", "text"]),
            })),
        },
        {
            "name": "cut_marks",
            "description": "Repères de découpe autour de chaque bloc (groupez d'abord une étiquette) : corners, dashed (ciseaux), none (retire).",
            "inputSchema": obj(&["element_ids", "style"], json!({ "element_ids": ids, "style": one_of(&["corners", "dashed", "none"]) })),
        },
        {
            "name": "add_caption",
            "description": "Légende sous une photo, groupée avec elle.",
            "inputSchema": obj(&["photo_id"], json!({ "photo_id": id, "content": text })),
        },
        {
            "name": "decor",
            "description": "page_frame (style), frieze (frise des marges : side, motifs), block_frieze (autour de element_id), remove_frieze (element_id d'une de ses images). motifs : mots-clés d'images ; size_mm 15, gap_mm 3.",
            "inputSchema": obj(&["kind"], json!({
                "kind": one_of(&["page_frame", "frieze", "block_frieze", "remove_frieze"]), "page": int, "element_id": id,
                "style": one_of(&["thin", "thick", "rounded", "double"]), "side": one_of(&["top", "bottom", "around"]),
                "motifs": ids, "size_mm": num, "gap_mm": num,
            })),
        },
        {
            "name": "exercise",
            "description": "Fiche sous le contenu : graphisme (motif, rows, dots, guides, arrow), maze, symmetry (figure) ; à x_mm, y_mm : dotted_path (path), start_point, direction_arrow.",
            "inputSchema": obj(&["kind", "page"], json!({
                "kind": one_of(&["graphisme", "maze", "symmetry", "dotted_path", "start_point", "direction_arrow"]),
                "page": int, "level": one_of(&["PS", "MS", "GS"]),
                "motif": one_of(&["ponts", "coupes", "vagues", "boucles", "creneaux", "dents", "traits", "ronds"]),
                "rows": int, "dots": one_of(&["tout", "moitie", "quelques", "seul", "progressif"]),
                "guides": one_of(&["base", "couloir", "aucune"]), "arrow": boolean, "figure": one_of(&["sapin", "maison", "fusee", "bateau", "coeur"]),
                "path": one_of(&["descend", "monte", "vague", "arc"]), "x_mm": num, "y_mm": num,
            })),
        },
        {
            "name": SAVE_AS_TEMPLATE,
            "description": "Enregistre une page comme modèle, seulement sur demande. Cahutin le remplit avec la classe : champs, et photo ou symbole de l'élève dans chaque case photo vide. \
Étiquettes : cadre et champs groupés, puis cut_marks et repeat (un élève par bloc). Lettres mobiles : {PRENOM_1}, {PRENOM_2}… par case.",
            "inputSchema": obj(&["page", "name"], json!({ "page": int, "name": text })),
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un outil décrit mais oublié dans `MUTATING` ne ferait aucune étape, et le serveur n'écrirait pas le fichier.
    #[test]
    fn every_described_tool_is_wired() {
        let names: Vec<String> = definitions().as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
        for n in ["set_look", "arrange", "group", "ungroup", "set_page_format", "add_table", "table_edit", "repeat", "set_template_role", "page_grid", "cut_marks", "add_caption", "decor", "exercise"] {
            assert!(names.iter().any(|x| x == n), "{n} n'est pas décrit");
        }
        for n in names.iter().filter(|n| *n != "get_layout" && *n != SAVE_AS_TEMPLATE) {
            assert!(MUTATING.contains(&n.as_str()), "{n} manque dans MUTATING");
            let err = apply(&mut Document::new("x"), n, &json!({}), &[]).err().unwrap_or_default();
            assert!(!err.starts_with("outil inconnu"), "{n} : {err}");
        }
    }
}

#[cfg(test)]
mod size {
    /// Les descriptions des outils partent à chaque demande, et chaque échange les relit : un petit modèle
    /// local (Ollama, 16 000 jetons) doit garder la place de la page et de la conversation.
    #[test]
    fn the_tool_descriptions_stay_small() {
        let n = serde_json::to_string(&super::definitions()).unwrap().len();
        assert!(n <= 10_500, "{n} octets");
    }
}
