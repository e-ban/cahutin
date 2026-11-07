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

/// Description des outils (format MCP : `name`, `description`, `inputSchema`).
pub fn definitions() -> Value {
    let id = json!({ "type": "string", "description": "Identifiant de l'élément (champ id de get_layout)" });
    let mm = |d: &str| json!({ "type": "number", "description": d });
    let page = json!({ "type": "integer", "description": "Indice de page (0 pour la première)" });
    let color = json!({ "type": "string", "description": "#rrggbb" });
    let frame = |extra: Value, required: &[&str]| {
        let mut props = json!({ "page": page, "x_mm": mm("Bord gauche"), "y_mm": mm("Bord haut"), "w_mm": mm("Largeur"), "h_mm": mm("Hauteur") });
        props.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        let mut req = vec!["page", "x_mm", "y_mm", "w_mm", "h_mm"];
        req.extend_from_slice(required);
        json!({ "type": "object", "required": req, "properties": props })
    };
    let font = json!({ "type": "string" });
    let size = json!({ "type": "number", "description": "Taille en points" });
    let align = json!({ "type": "string", "enum": ["left", "center", "right", "justify"] });
    let valign = json!({ "type": "string", "enum": ["top", "middle", "bottom"] });
    let style = json!({ "type": "string", "description": "Identifiant d'un style nommé (titre, consigne, texte, prenom…)" });
    let field = json!({ "type": "string", "description": format!("Champ rempli avec la classe : {}", template::fields_hint()) });
    let lines = json!({ "type": "string", "enum": ["seyes", "small_squares", "lines", "dots", "none"], "description": "Lignes d'écriture sous le texte" });
    json!([
        {
            "name": "get_layout",
            "description": "Mise en page du document : pages, puis chaque élément (texte, photo, forme, image) avec son cadre en mm, son style et s'il est sélectionné. Ni texte ni photo.",
            "inputSchema": { "type": "object", "properties": { "page": { "type": "integer", "description": "Indice de page (0 pour la première) ; absent : toutes" } } },
        },
        {
            "name": "set_frame",
            "description": "Déplace, redimensionne ou tourne un élément. Seules les valeurs données changent.",
            "inputSchema": { "type": "object", "required": ["element_id"], "properties": {
                "element_id": id, "x_mm": mm("Bord gauche"), "y_mm": mm("Bord haut"), "w_mm": mm("Largeur"), "h_mm": mm("Hauteur"), "rotation_deg": mm("Rotation en degrés"),
            } },
        },
        {
            "name": "set_text_style",
            "description": "Change l'allure d'un texte (jamais son contenu) : police, taille, couleur, alignement, style nommé (text_styles de get_layout), lettres creuses, lignes d'écriture.",
            "inputSchema": { "type": "object", "required": ["element_id"], "properties": {
                "element_id": id, "font": font, "size_pt": size, "color": color, "align": align,
                "valign": valign, "style": style, "outline": { "type": "boolean", "description": "Lettres creuses, à repasser" }, "writing_lines": lines,
            } },
        },
        {
            "name": "set_look",
            "description": "Change le remplissage, le bord et les coins d'une forme, d'une case photo ou du cadre d'un texte ; l'ombre d'une case photo.",
            "inputSchema": { "type": "object", "required": ["element_id"], "properties": {
                "element_id": id,
                "fill": { "type": "string", "description": "#rrggbb, ou none pour aucun" },
                "border_color": color, "border_width_mm": mm("Épaisseur du bord"),
                "border_style": { "type": "string", "enum": ["solid", "dashed", "dotted", "none"] },
                "corner_radius_mm": mm("Rayon des coins arrondis"),
                "shadow": { "type": "boolean" },
            } },
        },
        {
            "name": "set_background",
            "description": "Change le fond d'une page : couleur et motif d'école. Seules les valeurs données changent.",
            "inputSchema": { "type": "object", "required": ["page"], "properties": {
                "page": page,
                "color": color,
                "pattern": { "type": ["string", "null"], "enum": ["seyes", "small_squares", "lines", "dots", null] },
            } },
        },
        {
            "name": "add_text",
            "description": "Ajoute un texte sur une page ; avec « field » (au lieu de « content »), une zone vide que Cahutin remplira avec la classe.",
            "inputSchema": frame(json!({ "content": { "type": "string" }, "field": field, "style": style, "font": font, "size_pt": size, "color": color, "align": align, "valign": valign }), &["content"]),
        },
        {
            "name": "set_text",
            "description": "Remplace le texte d'un élément. Vous ne pouvez pas lire le texte actuel.",
            "inputSchema": { "type": "object", "required": ["element_id", "content"], "properties": { "element_id": id, "content": { "type": "string" } } },
        },
        {
            "name": "add_shape",
            "description": "Ajoute une forme sur une page.",
            "inputSchema": frame(json!({
                "shape": { "type": "string", "enum": ["rect", "ellipse", "line", "arrow", "star", "heart", "bubble", "cloud"] },
                "fill": color, "border_color": color, "border_width_mm": mm("Épaisseur du bord"),
            }), &["shape"]),
        },
        {
            "name": "add_image",
            "description": "Ajoute une image de la banque, trouvée par un mot-clé en français (« tulipe », « soleil »).",
            "inputSchema": frame(json!({ "query": { "type": "string", "description": "Mot-clé en français" }, "to_color": { "type": "boolean", "description": "Contour noir seul, à colorier" } }), &["query"]),
        },
        {
            "name": "add_photo_zone",
            "description": "Ajoute une case photo vide, que l'utilisateur remplira.",
            "inputSchema": frame(json!({}), &[]),
        },
        {
            "name": "delete_element",
            "description": "Supprime un élément.",
            "inputSchema": { "type": "object", "required": ["element_id"], "properties": { "element_id": id } },
        },
        {
            "name": "add_page",
            "description": "Ajoute une page vide à la fin du document ; rend son indice.",
            "inputSchema": { "type": "object", "properties": {} },
        },
        {
            "name": "arrange",
            "description": "Met un élément (avec tout son groupe) devant ou derrière les autres, le duplique (rend les nouveaux identifiants) ou le retourne en miroir.",
            "inputSchema": { "type": "object", "required": ["element_id", "action"], "properties": {
                "element_id": id, "action": { "type": "string", "enum": ["front", "back", "duplicate", "mirror_h", "mirror_v"] },
            } },
        },
        {
            "name": "group",
            "description": "Groupe des éléments d'une même page, pour qu'ils se déplacent ensemble.",
            "inputSchema": { "type": "object", "required": ["element_ids"], "properties": { "element_ids": { "type": "array", "items": { "type": "string" } } } },
        },
        {
            "name": "ungroup",
            "description": "Dissocie le groupe d'un élément (un seul niveau).",
            "inputSchema": { "type": "object", "required": ["element_id"], "properties": { "element_id": id } },
        },
        {
            "name": "set_page_format",
            "description": "Change le format d'une page (le contenu est agrandi ou réduit d'autant) ou son orientation.",
            "inputSchema": { "type": "object", "required": ["page"], "properties": {
                "page": page, "format": { "type": "string", "enum": ["A5", "A4", "A3"] }, "orientation": { "type": "string", "enum": ["portrait", "landscape"] },
            } },
        },
        {
            "name": "add_table",
            "description": "Ajoute un tableau de cases texte vides ; rend son id et ses cases (element_id, col, row). Écrivez dans une case avec set_text.",
            "inputSchema": frame(json!({ "cols": { "type": "integer", "description": "Colonnes (1 à 50)" }, "rows": { "type": "integer", "description": "Lignes (1 à 50)" } }), &["cols", "rows"]),
        },
        {
            "name": "table_edit",
            "description": "Modifie un tableau à partir d'une de ses cases : ajoute ou retire une ligne ou une colonne, fusionne avec une autre case, sépare une case fusionnée, ou change son contenu (texte, image de la banque, photo).",
            "inputSchema": { "type": "object", "required": ["cell_id", "action"], "properties": {
                "cell_id": { "type": "string", "description": "Identifiant d'une case (cells de get_layout)" },
                "action": { "type": "string", "enum": ["insert_row_above", "insert_row_below", "insert_col_left", "insert_col_right", "remove_row", "remove_col", "merge", "split", "set_content"] },
                "other_cell_id": { "type": "string", "description": "Pour merge : l'autre case" },
                "content": { "type": "string", "enum": ["text", "image", "photo"], "description": "Pour set_content" },
                "query": { "type": "string", "description": "Pour une image : mot-clé en français" },
            } },
        },
        {
            "name": "repeat",
            "description": "Répète un bloc (les éléments donnés, avec leurs groupes) en grille sur sa page, pour une planche d'étiquettes ou de cartes : cols × rows exemplaires, lui compris, ou fill_page pour en mettre autant que la page en tient. Chaque exemplaire forme un groupe.",
            "inputSchema": { "type": "object", "required": ["element_ids"], "properties": {
                "element_ids": { "type": "array", "items": { "type": "string" } },
                "cols": { "type": "integer", "description": "1 à 20" }, "rows": { "type": "integer", "description": "1 à 20" },
                "fill_page": { "type": "boolean" },
                "gap_mm": mm("Espace entre les exemplaires (5 par défaut)"),
            } },
        },
        {
            "name": "set_template_role",
            "description": "Rôle d'un texte ou d'une photo dans un modèle : fill (zone vidée à chaque usage), fixed (gardé tel quel et verrouillé : à faire en dernier), none (pas gardé dans le modèle). Avec « field », le texte devient ce champ de la classe, gardé tel quel ; un texte qui contient un champ n'a pas de rôle.",
            "inputSchema": { "type": "object", "required": ["element_id", "role"], "properties": {
                "element_id": id, "role": { "type": "string", "enum": ["fill", "fixed", "none"] }, "field": field,
            } },
        },
        {
            "name": "page_grid",
            "description": "Grille de cases d'une page (pour un modèle) : set la pose (cases photo vides), merge et split fusionnent ou séparent des cases, set_kind change une case en photo ou texte, remove la retire en gardant les cases.",
            "inputSchema": { "type": "object", "required": ["page", "action"], "properties": {
                "page": page,
                "action": { "type": "string", "enum": ["set", "merge", "split", "set_kind", "remove"] },
                "cols": { "type": "integer", "description": "1 à 10" }, "rows": { "type": "integer", "description": "1 à 10" },
                "cell_id": id, "other_cell_id": id,
                "kind": { "type": "string", "enum": ["photo", "text"] },
            } },
        },
        {
            "name": "cut_marks",
            "description": "Repères de découpe autour de chaque bloc donné (un groupe, ou un élément seul : groupez d'abord une étiquette) : corners (traits aux coins), dashed (pointillés et ciseaux), none (les retirer).",
            "inputSchema": { "type": "object", "required": ["element_ids", "style"], "properties": {
                "element_ids": { "type": "array", "items": { "type": "string" } }, "style": { "type": "string", "enum": ["corners", "dashed", "none"] },
            } },
        },
        {
            "name": "add_caption",
            "description": "Ajoute une légende sous une photo, groupée avec elle ; « content » l'écrit.",
            "inputSchema": { "type": "object", "required": ["photo_id"], "properties": { "photo_id": id, "content": { "type": "string" } } },
        },
        {
            "name": "decor",
            "description": "Décors : page_frame (cadre de page, style), frieze (frise le long des marges : side, motifs), block_frieze (frise autour d'un élément : element_id, motifs), remove_frieze (element_id d'une image de la frise).",
            "inputSchema": { "type": "object", "required": ["kind"], "properties": {
                "kind": { "type": "string", "enum": ["page_frame", "frieze", "block_frieze", "remove_frieze"] },
                "page": page, "element_id": id,
                "style": { "type": "string", "enum": ["thin", "thick", "rounded", "double"] },
                "side": { "type": "string", "enum": ["top", "bottom", "around"] },
                "motifs": { "type": "array", "items": { "type": "string" }, "description": "Mots-clés français des images, répétées dans l'ordre" },
                "size_mm": mm("Taille des images (15 par défaut)"), "gap_mm": mm("Écart minimal (3 par défaut)"),
            } },
        },
        {
            "name": "exercise",
            "description": "Fiches, posées sous ce que la page contient : graphisme (lignes à repasser : motif, rows), maze (labyrinthe), symmetry (dessin à compléter : figure) ; et, à x_mm/y_mm : dotted_path (chemin à suivre entre deux images : path), start_point (point vert de départ), direction_arrow (flèche de sens). level : PS, MS ou GS.",
            "inputSchema": { "type": "object", "required": ["kind", "page"], "properties": {
                "kind": { "type": "string", "enum": ["graphisme", "maze", "symmetry", "dotted_path", "start_point", "direction_arrow"] },
                "page": page, "level": { "type": "string", "enum": ["PS", "MS", "GS"] },
                "motif": { "type": "string", "enum": ["ponts", "coupes", "vagues", "boucles", "creneaux", "dents", "traits", "ronds"] },
                "rows": { "type": "integer" },
                "figure": { "type": "string", "enum": ["sapin", "maison", "fusee", "bateau", "coeur"] },
                "path": { "type": "string", "enum": ["descend", "monte", "vague", "arc"] },
                "x_mm": mm("Bord gauche"), "y_mm": mm("Bord haut"),
            } },
        },
        {
            "name": SAVE_AS_TEMPLATE,
            "description": "Enregistre une page comme modèle dans le tiroir Modèles. Seulement si l'utilisateur demande un modèle. \
Le modèle garde les formes, les images, les zones à remplir, les éléments fixes et les textes que vous avez écrits. \
Cahutin le remplit avec la classe : les champs entre accolades, sans accents, dans n'importe quel texte (add_text avec field, ou {prenom}, {PRENOM}, {nom}, {groupe}, {age}, {anniversaire} écrits dans un texte, \
{anniversaires_janvier}… pour une affiche de toute la classe), et chaque case photo vide reçoit la photo de l'élève ou son symbole. \
La date ({date}, {date_courte}, {jour_semaine}, {annee_scolaire}…) et la classe ({classe}, {niveau}, {ecole}, {enseignant}, {nombre_eleves}) s'écrivent une fois, quand une page est créée depuis le modèle. \
Pour des étiquettes : un cadre et ses champs, groupés, puis cut_marks et repeat : un élève par bloc (class_fill de get_layout dit « sheet »). \
Lettres mobiles : un champ {PRENOM_1}, {PRENOM_2}… par case.",
            "inputSchema": { "type": "object", "required": ["page", "name"], "properties": { "page": page, "name": { "type": "string", "description": "Nom du modèle" } } },
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
    /// Les descriptions des outils partent à chaque demande : un petit modèle local (Ollama, 16 000 jetons)
    /// doit garder la place de la page et de la conversation.
    #[test]
    fn the_tool_descriptions_stay_small() {
        let n = serde_json::to_string(&super::definitions()).unwrap().len();
        assert!(n < 18_000, "{n} octets");
    }
}
