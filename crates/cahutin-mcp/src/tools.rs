//! Les outils de l'assistant : leur description pour le modèle, et leur application à un document en
//! mémoire. Partagés par le serveur MCP et par l'application.

use crate::{edit, view};
use carnet_core::{Document, Id};
use serde_json::{Value, json};

/// Outils qui modifient le document (chacun fait une étape de la proposition).
pub const MUTATING: &[&str] = &["set_frame", "set_text_style", "set_background", "add_text", "set_text", "add_shape", "add_image", "add_photo_zone", "delete_element", "add_page"];

/// Outil traité par la boucle de dialogue, pas par `apply` : il n'agit qu'à l'acceptation.
pub const SAVE_AS_TEMPLATE: &str = "save_as_template";

pub fn apply(doc: &mut Document, name: &str, args: &Value, selection: &[Id]) -> Result<Value, String> {
    match name {
        "get_layout" => Ok(view::layout(doc, args.get("page").and_then(Value::as_u64).map(|p| p as usize), selection)),
        "set_frame" => edit::set_frame(doc, args),
        "set_text_style" => edit::set_text_style(doc, args),
        "set_background" => edit::set_background(doc, args),
        "add_text" => edit::add_text(doc, args),
        "set_text" => edit::set_text(doc, args),
        "add_shape" => edit::add_shape(doc, args),
        "add_image" => edit::add_image(doc, args),
        "add_photo_zone" => edit::add_photo_zone(doc, args),
        "delete_element" => edit::delete_element(doc, args),
        "add_page" => edit::add_page(doc, args),
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
            "description": "Change la police, la taille, la couleur ou l'alignement d'un texte (jamais son contenu).",
            "inputSchema": { "type": "object", "required": ["element_id"], "properties": {
                "element_id": id, "font": font, "size_pt": size, "color": color, "align": align,
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
            "description": "Ajoute un texte sur une page.",
            "inputSchema": frame(json!({ "content": { "type": "string" }, "font": font, "size_pt": size, "color": color, "align": align }), &["content"]),
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
            "inputSchema": frame(json!({ "query": { "type": "string", "description": "Mot-clé en français" } }), &["query"]),
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
            "name": SAVE_AS_TEMPLATE,
            "description": "Enregistre une page comme modèle dans le tiroir Modèles. Seulement si l'utilisateur demande un modèle.",
            "inputSchema": { "type": "object", "required": ["page", "name"], "properties": { "page": page, "name": { "type": "string", "description": "Nom du modèle" } } },
        },
    ])
}
