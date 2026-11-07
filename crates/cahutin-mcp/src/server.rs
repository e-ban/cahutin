//! Protocole MCP sur stdio (JSON-RPC 2.0, un message par ligne) : `initialize`, `tools/list`,
//! `tools/call`, `ping`. Chaque appel relit le fichier, pour suivre ce que l'utilisateur a enregistré.

use crate::{file, tools};
use serde_json::{Value, json};
use std::path::PathBuf;

pub struct Server {
    pub path: PathBuf,
}

pub const INSTRUCTIONS: &str = "Mise en page d'un document Cahutin (pages de classe pour l'école primaire). \
Unités en millimètres, origine en haut à gauche de la page (A4 portrait : 210 × 297). \
Vous ne voyez ni les textes ni les photos, seulement leurs cadres et leurs styles : « empty » dit si un texte \
est vide, « has_photo » si une case photo est remplie. Un élément verrouillé (« locked ») ne se modifie pas. \
Polices fournies : Andika, Atkinson Hyperlegible, OpenDyslexic, Playwrite FR Moderne (cursive).";

impl Server {
    /// Réponse à un message, ou rien pour une notification.
    pub fn handle(&self, msg: &Value) -> Option<Value> {
        let id = msg.get("id")?.clone();
        let params = msg.get("params").cloned().unwrap_or(json!({}));
        let result = match msg.get("method").and_then(Value::as_str).unwrap_or("") {
            "initialize" => Ok(json!({
                "protocolVersion": params.get("protocolVersion").cloned().unwrap_or(json!("2025-06-18")),
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "cahutin-mcp", "version": env!("CARGO_PKG_VERSION") },
                "instructions": INSTRUCTIONS,
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools() })),
            "tools/call" => Ok(self.call(&params)),
            other => Err(json!({ "code": -32601, "message": format!("méthode inconnue : {other}") })),
        };
        Some(match result {
            Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
            Err(e) => json!({ "jsonrpc": "2.0", "id": id, "error": e }),
        })
    }

    fn call(&self, params: &Value) -> Value {
        let args = params.get("arguments").cloned().unwrap_or(json!({}));
        let name = params.get("name").and_then(Value::as_str).unwrap_or("");
        let outcome = if name == "get_layout" {
            file::read(&self.path).and_then(|mut doc| tools::apply(&mut doc, name, &args, &[]))
        } else if tools::MUTATING.contains(&name) {
            self.edit(name, &args)
        } else {
            Err(format!("outil inconnu : {name}"))
        };
        let (text, is_error) = match outcome {
            Ok(v) => (v.to_string(), false),
            Err(e) => (e, true),
        };
        json!({ "content": [{ "type": "text", "text": text }], "isError": is_error })
    }

    /// Relit le fichier, applique la modification et l'écrit.
    fn edit(&self, name: &str, args: &Value) -> Result<Value, String> {
        let mut doc = file::read(&self.path)?;
        let mut out = tools::apply(&mut doc, name, args, &[])?;
        file::write(&self.path, &doc)?;
        out["note"] = json!("Enregistré. Rouvrir le document dans Cahutin pour voir le changement.");
        Ok(out)
    }
}

/// Les outils du serveur : tous, sauf l'enregistrement d'un modèle (pas de bibliothèque de modèles ici).
fn tools() -> Value {
    let all = tools::definitions();
    Value::Array(all.as_array().unwrap().iter().filter(|t| t["name"] != tools::SAVE_AS_TEMPLATE).cloned().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use carnet_core::*;

    fn server() -> (tempfile::TempDir, Server) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prenom.carnet");
        let doc = examples::document("prenom").unwrap();
        native::save_to_path(&doc, &photo::AssetStore::default(), &fonts::FontSet::builtin(), &path).unwrap();
        (dir, Server { path })
    }

    fn call(s: &Server, id: u64, method: &str, params: Value) -> Value {
        s.handle(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })).unwrap()
    }

    #[test]
    fn initialize_announces_the_tools_capability() {
        let (_d, s) = server();
        let r = call(&s, 1, "initialize", json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "essai", "version": "1" } }));
        assert_eq!(r["id"], 1);
        assert_eq!(r["result"]["protocolVersion"], "2025-06-18");
        assert!(r["result"]["capabilities"]["tools"].is_object());
        assert_eq!(r["result"]["serverInfo"]["name"], "cahutin-mcp");
        assert!(s.handle(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })).is_none());
    }

    #[test]
    fn the_tools_are_listed_with_their_schema() {
        let (_d, s) = server();
        let r = call(&s, 2, "tools/list", json!({}));
        let names: Vec<_> = r["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
        assert_eq!(
            names,
            [
                "get_layout",
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
                "repeat"
            ]
        );
        assert!(r["result"]["tools"].as_array().unwrap().iter().all(|t| t["inputSchema"]["type"] == "object"));
    }

    #[test]
    fn get_layout_returns_the_page_without_its_text() {
        let (_d, s) = server();
        let r = call(&s, 3, "tools/call", json!({ "name": "get_layout", "arguments": {} }));
        assert_eq!(r["result"]["isError"], false);
        let text = r["result"]["content"][0]["text"].as_str().unwrap();
        let layout: Value = serde_json::from_str(text).unwrap();
        assert_eq!(layout["page_count"], 1);
        assert!(!text.contains("Léa") && !text.contains("J'écris"), "{text}");
    }

    #[test]
    fn set_frame_writes_the_file() {
        let (_d, s) = server();
        let id = file::read(&s.path).unwrap().pages[0].elements.iter().find(|e| !e.locked).unwrap().id.clone();
        let r = call(&s, 4, "tools/call", json!({ "name": "set_frame", "arguments": { "element_id": id, "x_mm": 42 } }));
        assert_eq!(r["result"]["isError"], false, "{r}");
        let doc = file::read(&s.path).unwrap();
        assert_eq!(doc.pages[0].elements.iter().find(|e| e.id == id).unwrap().frame.x_mm, 42.0);
    }

    #[test]
    fn errors_are_reported_to_the_assistant() {
        let (_d, s) = server();
        let r = call(&s, 5, "tools/call", json!({ "name": "set_frame", "arguments": { "element_id": "inconnu", "x_mm": 1 } }));
        assert_eq!(r["result"]["isError"], true);
        let r = call(&s, 6, "tools/call", json!({ "name": "delete_everything", "arguments": {} }));
        assert_eq!(r["result"]["isError"], true);
        let r = call(&s, 7, "resources/list", json!({}));
        assert_eq!(r["error"]["code"], -32601);
    }
}
