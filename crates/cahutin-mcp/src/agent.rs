//! Dialogue avec le modèle (format « compatible OpenAI ») : le modèle appelle les outils sur une copie du
//! document ; chaque modification est une étape de la proposition, envoyée tout de suite à l'interface.

use crate::{describe, server::INSTRUCTIONS, tools};
use carnet_core::{Document, Id};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};
use ts_rs::TS;

pub const MAX_TOOL_CALLS: usize = 25;

/// Une demande : le document tel qu'il est affiché (la proposition en cours, s'il y en a une), la page vue,
/// la sélection, les échanges précédents de la proposition et le nouveau message.
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, rename = "AssistantRequest")]
pub struct Request {
    pub document: Document,
    pub page: usize,
    pub selection: Vec<Id>,
    #[ts(type = "Array<unknown>")]
    pub history: Vec<Value>,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export, rename = "AssistantTemplateRequest")]
pub struct TemplateRequest {
    pub name: String,
    pub page: usize,
}

/// Une modification : le document après elle, ce qu'elle a changé, et les textes que l'assistant a écrits.
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, rename = "AssistantStep")]
pub struct Step {
    pub document: Document,
    pub summary: Vec<String>,
    pub touched: Vec<Id>,
    pub written: Vec<Id>,
    pub template: Option<TemplateRequest>,
}

#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, rename = "AssistantDone")]
pub struct Done {
    pub reply: String,
    /// Les échanges de cette demande et des précédentes, sans la consigne : à renvoyer pour continuer.
    #[ts(type = "Array<unknown>")]
    pub history: Vec<Value>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, rename = "AssistantEvent")]
#[serde(tag = "type")]
pub enum Event {
    Step(Step),
    Done(Done),
}

#[derive(Debug, PartialEq)]
pub enum ChatError {
    Unreachable,
    /// Pas de réponse dans le délai.
    TimedOut,
    KeyRefused,
    ModelUnknown,
    /// Trop de demandes, ou crédit épuisé (code 429).
    Busy,
    /// Le service est en panne (codes 500 et plus).
    ServiceDown,
    /// L'utilisatrice a cliqué « Arrêter » pendant que le modèle répondait.
    Cancelled,
    Other(String),
}

impl ChatError {
    pub fn message_fr(&self) -> String {
        match self {
            ChatError::Unreachable => "Je n'arrive pas à joindre l'assistant. Vérifiez la connexion à Internet et les réglages de l'assistant (Préférences).".into(),
            ChatError::TimedOut => "L'assistant met trop de temps à répondre. Réessayez, ou demandez une modification plus simple.".into(),
            ChatError::KeyRefused => "Le service refuse la clé. Vérifiez-la dans les Préférences.".into(),
            ChatError::ModelUnknown => "Ce modèle d'IA est introuvable : vérifiez les réglages de l'assistant (Préférences).".into(),
            ChatError::Busy => "Le service de l'assistant est trop sollicité, ou votre crédit est épuisé. Réessayez dans quelques minutes.".into(),
            ChatError::ServiceDown => "Le service de l'assistant a un problème en ce moment. Réessayez plus tard.".into(),
            ChatError::Cancelled => "Arrêté.".into(),
            ChatError::Other(d) => format!("L'assistant a rencontré un problème ({d})."),
        }
    }
}

/// Outils au format OpenAI.
pub fn openai_tools() -> Value {
    let defs = tools::definitions();
    Value::Array(
        defs.as_array()
            .unwrap()
            .iter()
            .map(|t| json!({ "type": "function", "function": { "name": t["name"], "description": t["description"], "parameters": t["inputSchema"] } }))
            .collect(),
    )
}

fn system_prompt(req: &Request) -> String {
    let selection = if req.selection.is_empty() { "aucun".to_string() } else { req.selection.join(", ") };
    format!(
        "{INSTRUCTIONS} Page affichée : {} (indice). Éléments sélectionnés : {selection}. \
Commencez toujours par get_layout pour trouver les identifiants, puis appelez les outils de modification. \
N'affirmez jamais une modification sans avoir appelé l'outil. Vous ne pouvez pas lire les textes : vous pouvez en écrire. \
N'utilisez save_as_template que si l'utilisateur demande un modèle. Répondez en français.",
        req.page
    )
}

/// Arguments d'un appel : une chaîne JSON (API OpenAI) ou déjà un objet.
fn arguments(raw: &Value) -> Result<Value, String> {
    match raw {
        Value::String(s) if s.trim().is_empty() => Ok(json!({})),
        Value::String(s) => serde_json::from_str(s).map_err(|_| "arguments illisibles (JSON attendu)".to_string()),
        Value::Null => Ok(json!({})),
        v => Ok(v.clone()),
    }
}

/// Message du modèle tel qu'on le renvoie : sans `tool_calls` vide, arguments toujours en chaîne JSON
/// (ce qu'exige l'API OpenAI, même si le serveur les a rendus en objet).
fn assistant_message(content: &str, tool_calls: &[Value]) -> Value {
    let mut m = json!({ "role": "assistant", "content": content });
    if !tool_calls.is_empty() {
        let calls: Vec<Value> = tool_calls
            .iter()
            .map(|tc| {
                let mut tc = tc.clone();
                let args = &tc["function"]["arguments"];
                if !args.is_string() {
                    tc["function"]["arguments"] = json!(args.to_string());
                }
                tc
            })
            .collect();
        m["tool_calls"] = Value::Array(calls);
    }
    m
}

/// Avertissement pour le modèle si un élément touché sort de sa page (tolérance d'un millimètre).
fn off_page(doc: &Document, ids: &[Id]) -> Option<String> {
    for page in &doc.pages {
        let (w, h) = (page.size.width_mm, page.size.height_mm);
        for e in page.elements.iter().filter(|e| ids.contains(&e.id)) {
            let f = &e.frame;
            if f.x_mm < -1.0 || f.y_mm < -1.0 || f.x_mm + f.w_mm > w + 1.0 || f.y_mm + f.h_mm > h + 1.0 {
                return Some(format!("l'élément {} dépasse de la page ({w} × {h} mm) : corrigez son cadre", e.id));
            }
        }
    }
    None
}

/// Enregistrer un modèle : rien ne change avant l'acceptation.
fn template_request(doc: &Document, args: &Value) -> Result<TemplateRequest, String> {
    let page = args.get("page").and_then(Value::as_u64).ok_or("« page » manque")? as usize;
    if page >= doc.pages.len() {
        return Err(format!("aucune page {page}"));
    }
    let name = args.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()).ok_or("« name » manque")?;
    Ok(TemplateRequest { name: name.to_string(), page })
}

/// Fait la demande jusqu'à la réponse du modèle ; émet une `Step` par modification et toujours un `Done` à la fin.
/// `chat` reçoit le corps de la requête (sans `model`) et rend le message du premier choix.
pub fn run(req: Request, chat: &mut dyn FnMut(&Value) -> Result<Value, ChatError>, on_event: &mut dyn FnMut(Event), cancel: &AtomicBool) {
    let system = json!({ "role": "system", "content": system_prompt(&req) });
    let mut history = req.history.clone();
    history.push(json!({ "role": "user", "content": req.text }));
    let mut doc = req.document;
    let tools = openai_tools();
    let mut calls = 0;
    let finish = |reply: String, history: Vec<Value>, error: Option<String>, on_event: &mut dyn FnMut(Event)| on_event(Event::Done(Done { reply, history, error }));
    loop {
        if cancel.load(Ordering::SeqCst) {
            return finish(String::new(), history, Some("Arrêté.".into()), on_event);
        }
        let mut messages = vec![system.clone()];
        messages.extend(history.iter().cloned());
        let body = json!({ "messages": messages, "tools": tools, "tool_choice": "auto" });
        let message = match chat(&body) {
            Ok(m) => m,
            Err(e) => return finish(String::new(), history, Some(e.message_fr()), on_event),
        };
        let tool_calls = message.get("tool_calls").and_then(Value::as_array).cloned().unwrap_or_default();
        let content = message.get("content").and_then(Value::as_str).unwrap_or("").to_string();
        history.push(assistant_message(&content, &tool_calls));
        if tool_calls.is_empty() {
            return finish(content, history, None, on_event);
        }
        let mut stopped = false;
        for tc in &tool_calls {
            let id = tc.get("id").cloned().unwrap_or(Value::Null);
            // Chaque appel reçoit une réponse, même non exécuté : l'API OpenAI refuse sinon l'historique renvoyé.
            if stopped || calls >= MAX_TOOL_CALLS {
                stopped = true;
                history.push(json!({ "role": "tool", "tool_call_id": id, "content": "Non exécuté : limite d'actions atteinte." }));
                continue;
            }
            calls += 1;
            let name = tc["function"]["name"].as_str().unwrap_or("");
            let result = arguments(&tc["function"]["arguments"]).and_then(|args| {
                if name == tools::SAVE_AS_TEMPLATE {
                    let t = template_request(&doc, &args)?;
                    on_event(Event::Step(Step {
                        document: doc.clone(),
                        summary: vec![format!("Modèle « {} » enregistré si vous acceptez", t.name)],
                        touched: vec![],
                        written: vec![],
                        template: Some(t),
                    }));
                    return Ok(json!({ "ok": true, "note": "sera enregistré si la proposition est acceptée" }));
                }
                let before = doc.clone();
                // Un outil qui échoue à mi-chemin (élément ajouté, puis style refusé) ne laisse rien derrière lui.
                let out = tools::apply(&mut doc, name, &args, &req.selection).inspect_err(|_| doc = before.clone())?;
                if tools::MUTATING.contains(&name) {
                    let written = match name {
                        "add_text" => out["element_id"].as_str().map(String::from).into_iter().collect(),
                        "add_caption" if args["content"].as_str().is_some_and(|c| !c.trim().is_empty()) => out["element_id"].as_str().map(String::from).into_iter().collect(),
                        "set_text" => args["element_id"].as_str().map(String::from).into_iter().collect(),
                        _ => vec![],
                    };
                    let touched = describe::touched(&before, &doc);
                    let off = off_page(&doc, &touched);
                    on_event(Event::Step(Step { summary: describe::describe(&before, &doc), touched, document: doc.clone(), written, template: None }));
                    if let Some(warning) = off {
                        let mut out = out;
                        out["attention"] = json!(warning);
                        return Ok(out);
                    }
                }
                Ok(out)
            });
            let text = match result {
                Ok(v) => v.to_string(),
                Err(e) => format!("Erreur : {e}"),
            };
            history.push(json!({ "role": "tool", "tool_call_id": id, "content": text }));
        }
        if stopped {
            return finish(content, history, Some(format!("J'ai arrêté après {MAX_TOOL_CALLS} actions. Vous pouvez accepter, refuser ou continuer.")), on_event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    fn doc() -> Document {
        let mut d = Document::new("essai");
        crate::edit::add_text(&mut d, &json!({ "page": 0, "x_mm": 10, "y_mm": 10, "w_mm": 50, "h_mm": 20, "content": "Bonjour" })).unwrap();
        d
    }

    fn request(d: &Document) -> Request {
        Request { document: d.clone(), page: 0, selection: vec![d.pages[0].elements[0].id.clone()], history: vec![], text: "Mets le titre plus bas.".into() }
    }

    fn call(name: &str, args: Value) -> Value {
        json!({ "role": "assistant", "content": null, "tool_calls": [{ "id": format!("c-{name}"), "type": "function", "function": { "name": name, "arguments": args } }] })
    }

    fn reply(text: &str) -> Value {
        json!({ "role": "assistant", "content": text })
    }

    /// Joue `script` dans l'ordre ; garde les corps reçus.
    fn run_script(req: Request, script: Vec<Result<Value, ChatError>>) -> (Vec<Event>, Vec<Value>) {
        let mut script = script.into_iter();
        let mut bodies = Vec::new();
        let mut events = Vec::new();
        let cancel = AtomicBool::new(false);
        run(
            req,
            &mut |body| {
                bodies.push(body.clone());
                script.next().unwrap_or_else(|| Ok(reply("fini")))
            },
            &mut |e| events.push(e),
            &cancel,
        );
        (events, bodies)
    }

    fn steps(events: &[Event]) -> Vec<&Step> {
        events.iter().filter_map(|e| if let Event::Step(s) = e { Some(s) } else { None }).collect()
    }

    fn done(events: &[Event]) -> &Done {
        match events.last() {
            Some(Event::Done(d)) => d,
            _ => panic!("pas de Done à la fin"),
        }
    }

    #[test]
    fn one_tool_call_then_reply() {
        let d = doc();
        let id = d.pages[0].elements[0].id.clone();
        let (events, _) = run_script(request(&d), vec![Ok(call("set_frame", json!({ "element_id": id, "y_mm": 40 }))), Ok(reply("C'est fait."))]);
        let s = steps(&events);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].document.pages[0].elements[0].frame.y_mm, 40.0);
        assert_eq!(s[0].summary, ["Texte : déplacé"]);
        assert_eq!(s[0].touched, [id]);
        assert_eq!(done(&events).reply, "C'est fait.");
        assert!(done(&events).error.is_none());
    }

    #[test]
    fn arguments_as_string_or_object() {
        let d = doc();
        let id = d.pages[0].elements[0].id.clone();
        let as_string = json!({ "element_id": id, "y_mm": 40 }).to_string();
        let (events, bodies) = run_script(request(&d), vec![Ok(call("set_frame", json!(as_string))), Ok(call("set_frame", json!("{pas du json"))), Ok(reply("ok"))]);
        assert_eq!(steps(&events).len(), 1);
        let last = bodies.last().unwrap()["messages"].as_array().unwrap().last().unwrap().clone();
        assert_eq!(last["role"], "tool");
        assert!(last["content"].as_str().unwrap().starts_with("Erreur"));
    }

    #[test]
    fn calls_are_sent_back_with_string_arguments() {
        let (_, bodies) = run_script(request(&doc()), vec![Ok(call("get_layout", json!({ "page": 0 }))), Ok(reply("ok"))]);
        let assistant = &bodies[1]["messages"][2];
        assert_eq!(assistant["tool_calls"][0]["function"]["arguments"], json!("{\"page\":0}"));
        let (events, _) = run_script(request(&doc()), vec![Ok(reply("ok"))]);
        assert!(done(&events).history.last().unwrap().get("tool_calls").is_none());
    }

    #[test]
    fn tool_error_is_given_back_to_the_model() {
        let (events, bodies) = run_script(request(&doc()), vec![Ok(call("set_frame", json!({ "element_id": "inconnu", "x_mm": 1 }))), Ok(reply("Désolé."))]);
        assert!(steps(&events).is_empty());
        let msgs = bodies[1]["messages"].as_array().unwrap();
        let tool = msgs.last().unwrap();
        assert_eq!((tool["role"].as_str(), tool["tool_call_id"].as_str()), (Some("tool"), Some("c-set_frame")));
        assert!(tool["content"].as_str().unwrap().contains("aucun élément"));
    }

    #[test]
    fn stops_after_25_tool_calls() {
        let script = (0..40).map(|_| Ok(call("get_layout", json!({})))).collect();
        let (events, bodies) = run_script(request(&doc()), script);
        assert_eq!(done(&events).error.as_deref(), Some("J'ai arrêté après 25 actions. Vous pouvez accepter, refuser ou continuer."));
        let tool_results = bodies.last().unwrap()["messages"].as_array().unwrap().iter().filter(|m| m["role"] == "tool").count();
        assert!(tool_results <= MAX_TOOL_CALLS, "{tool_results}");
    }

    #[test]
    fn the_history_after_the_limit_answers_every_call() {
        let many = json!({ "role": "assistant", "content": "", "tool_calls": (0..30).map(|i| json!({ "id": format!("c{i}"), "type": "function", "function": { "name": "get_layout", "arguments": "{}" } })).collect::<Vec<_>>() });
        let (events, _) = run_script(request(&doc()), vec![Ok(many)]);
        let h = &done(&events).history;
        let ids: Vec<_> = h.iter().filter(|m| m["role"] == "tool").map(|m| m["tool_call_id"].as_str().unwrap().to_string()).collect();
        assert_eq!(ids.len(), 30, "chaque appel a sa réponse");
        assert!(done(&events).error.is_some());
    }

    #[test]
    fn a_failed_call_leaves_the_document_unchanged() {
        let d = doc();
        let id = d.pages[0].elements[0].id.clone();
        let bad = call("add_text", json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 20, "h_mm": 9, "content": "x", "color": "red" }));
        let (events, _) = run_script(request(&d), vec![Ok(bad), Ok(call("set_frame", json!({ "element_id": id, "y_mm": 40 }))), Ok(reply("ok"))]);
        let s = steps(&events);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].document.pages[0].elements.len(), 1, "le texte raté n'est pas resté");
    }

    #[test]
    fn a_cancelled_request_says_stopped() {
        let (events, _) = run_script(request(&doc()), vec![Err(ChatError::Cancelled)]);
        assert_eq!(done(&events).error.as_deref(), Some("Arrêté."));
    }

    #[test]
    fn chat_error_keeps_the_steps() {
        let d = doc();
        let id = d.pages[0].elements[0].id.clone();
        let (events, _) = run_script(request(&d), vec![Ok(call("set_frame", json!({ "element_id": id, "y_mm": 40 }))), Err(ChatError::Unreachable)]);
        assert_eq!(steps(&events).len(), 1);
        assert_eq!(done(&events).error, Some(ChatError::Unreachable.message_fr()));
    }

    #[test]
    fn template_request_changes_nothing() {
        let d = doc();
        let (events, bodies) = run_script(request(&d), vec![Ok(call("save_as_template", json!({ "page": 0, "name": "Saisons" }))), Ok(reply("ok"))]);
        let s = steps(&events);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].document, d);
        assert_eq!(s[0].summary, ["Modèle « Saisons » enregistré si vous acceptez"]);
        assert_eq!(s[0].template.as_ref().map(|t| (t.name.as_str(), t.page)), Some(("Saisons", 0)));
        let tool = bodies[1]["messages"].as_array().unwrap().last().unwrap().clone();
        assert!(tool["content"].as_str().unwrap().contains("sera enregistré si la proposition est acceptée"));
        let (events, _) = run_script(request(&d), vec![Ok(call("save_as_template", json!({ "page": 9, "name": "X" }))), Ok(reply("ok"))]);
        assert!(steps(&events).is_empty());
    }

    #[test]
    fn an_element_off_the_page_is_reported_to_the_model() {
        let d = doc();
        let id = d.pages[0].elements[0].id.clone();
        let (_, bodies) = run_script(request(&d), vec![Ok(call("set_frame", json!({ "element_id": id, "x_mm": 180 }))), Ok(reply("ok"))]);
        let tool = bodies[1]["messages"].as_array().unwrap().last().unwrap().clone();
        assert!(tool["content"].as_str().unwrap().contains("dépasse de la page"), "{tool}");
    }

    #[test]
    fn written_ids_are_reported() {
        let d = doc();
        let (events, _) = run_script(request(&d), vec![Ok(call("add_text", json!({ "page": 0, "x_mm": 1, "y_mm": 1, "w_mm": 20, "h_mm": 9, "content": "Été" }))), Ok(reply("ok"))]);
        let s = steps(&events);
        assert_eq!(s[0].written.len(), 1);
        assert_eq!(s[0].written, s[0].touched);
    }

    #[test]
    fn a_written_caption_counts_as_written() {
        let mut d = doc();
        let photo = crate::edit::add_photo_zone(&mut d, &json!({ "page": 0, "x_mm": 20, "y_mm": 20, "w_mm": 50, "h_mm": 40 })).unwrap()["element_id"].as_str().unwrap().to_string();
        let calls = vec![Ok(call("add_caption", json!({ "photo_id": photo, "content": "La ferme" }))), Ok(call("add_caption", json!({ "photo_id": photo }))), Ok(reply("ok"))];
        let (events, _) = run_script(request(&d), calls);
        let s = steps(&events);
        assert_eq!((s.len(), s[0].written.len()), (1, 1));
    }

    #[test]
    fn history_continues_a_proposal() {
        let d = doc();
        let (events, _) = run_script(request(&d), vec![Ok(reply("Première réponse"))]);
        let history = done(&events).history.clone();
        assert_eq!(history.first().unwrap()["role"], "user");
        let mut next = request(&d);
        next.history = history;
        next.text = "Encore".into();
        let (_, bodies) = run_script(next, vec![Ok(reply("ok"))]);
        let msgs = bodies[0]["messages"].as_array().unwrap();
        let contents: Vec<_> = msgs.iter().map(|m| m["content"].as_str().unwrap_or("").to_string()).collect();
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(&contents[1..], ["Mets le titre plus bas.", "Première réponse", "Encore"]);
    }

    #[test]
    fn selection_is_in_the_system_prompt() {
        let d = doc();
        let id = d.pages[0].elements[0].id.clone();
        let (_, bodies) = run_script(request(&d), vec![Ok(reply("ok"))]);
        let system = bodies[0]["messages"][0]["content"].as_str().unwrap().to_string();
        assert!(system.contains(&id), "{system}");
        assert!(system.contains("N'affirmez jamais une modification sans avoir appelé l'outil."));
        assert!(system.ends_with("Répondez en français."), "{system}");
        assert_eq!(bodies[0]["tool_choice"], "auto");
        // Claude refuse temperature < 1 sur ses modèles récents : on ne l'envoie pas.
        assert!(bodies[0].get("temperature").is_none());
        assert!(bodies[0]["tools"].as_array().unwrap().iter().any(|t| t["function"]["name"] == "save_as_template"));
    }

    #[test]
    fn cancel_stops_before_the_next_request() {
        let cancel = AtomicBool::new(true);
        let mut events = Vec::new();
        let mut calls = 0;
        run(request(&doc()), &mut |_| { calls += 1; Ok(reply("x")) }, &mut |e| events.push(e), &cancel);
        assert_eq!(calls, 0);
        assert_eq!(done(&events).error.as_deref(), Some("Arrêté."));
    }
}
