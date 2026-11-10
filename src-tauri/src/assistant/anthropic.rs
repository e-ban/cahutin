//! Claude par son API Messages (et non par sa porte « compatible OpenAI ») : seule celle-ci garde en
//! cache la consigne et les outils, environ 8 600 tokens renvoyés à chaque échange. La boucle de l'agent
//! parle le format OpenAI ; ce module traduit la requête à l'aller et la réponse au retour.

use cahutin_mcp::agent::NATIVE;
use serde_json::{Value, json};

/// Version de l'API Messages envoyée dans l'en-tête `anthropic-version`.
pub const VERSION: &str = "2026-10-21";
/// Plafond de la réponse, réflexion comprise : une réponse de l'assistant tient en quelques phrases.
const MAX_TOKENS: u32 = 16_000;

/// L'adresse est celle du service de Claude.
pub fn is_anthropic(url: &str) -> bool {
    super::host_of(url).is_some_and(|(secure, host)| secure && host == "api.anthropic.com")
}

pub fn endpoint(url: &str) -> String {
    let base = url.trim().trim_end_matches('/');
    if base.ends_with("/v1") { format!("{base}/messages") } else { format!("{base}/v1/messages") }
}

fn ephemeral() -> Value {
    json!({ "type": "ephemeral" })
}

/// Un message du modèle (format OpenAI) en blocs Messages : sa réponse native quand il y en a une (blocs de
/// réflexion compris, à renvoyer tels quels), sinon son texte et ses appels d'outils.
fn assistant_blocks(m: &Value) -> Vec<Value> {
    if let Some(native) = m.get(NATIVE).and_then(Value::as_array) {
        return native.clone();
    }
    let mut blocks = Vec::new();
    if let Some(text) = m["content"].as_str().filter(|t| !t.trim().is_empty()) {
        blocks.push(json!({ "type": "text", "text": text }));
    }
    for tc in m["tool_calls"].as_array().into_iter().flatten() {
        let args = &tc["function"]["arguments"];
        let input = match args {
            Value::String(s) => serde_json::from_str(s).ok().filter(Value::is_object).unwrap_or_else(|| json!({})),
            Value::Object(_) => args.clone(),
            _ => json!({}),
        };
        blocks.push(json!({ "type": "tool_use", "id": tc["id"], "name": tc["function"]["name"], "input": input }));
    }
    blocks
}

/// La requête de l'agent (format OpenAI) en requête Messages. La consigne porte une marque de cache, qui
/// couvre aussi les outils placés avant elle ; le dernier message en porte une autre, pour que l'échange
/// suivant de la même demande relise tout ce qui précède. Les résultats d'outils qui se suivent forment un
/// seul message, comme l'API le demande.
pub fn request(body: &Value, model: &str) -> Value {
    let mut system = Vec::new();
    let mut messages: Vec<Value> = Vec::new();
    for m in body["messages"].as_array().into_iter().flatten() {
        match m["role"].as_str() {
            Some("system") => system.push(json!({ "type": "text", "text": m["content"].as_str().unwrap_or("") })),
            Some("user") => messages.push(json!({ "role": "user", "content": [{ "type": "text", "text": m["content"].as_str().unwrap_or("") }] })),
            Some("assistant") => {
                let blocks = assistant_blocks(m);
                if !blocks.is_empty() {
                    messages.push(json!({ "role": "assistant", "content": blocks }));
                }
            }
            Some("tool") => {
                let result = json!({ "type": "tool_result", "tool_use_id": m["tool_call_id"], "content": m["content"].as_str().unwrap_or("") });
                let previous_is_results = messages.last().is_some_and(|p| p["role"] == "user" && p["content"][0]["type"] == "tool_result");
                if previous_is_results {
                    messages.last_mut().unwrap()["content"].as_array_mut().unwrap().push(result);
                } else {
                    messages.push(json!({ "role": "user", "content": [result] }));
                }
            }
            _ => {}
        }
    }
    if let Some(last) = system.last_mut() {
        last["cache_control"] = ephemeral();
    }
    if let Some(block) = messages.last_mut().and_then(|m| m["content"].as_array_mut()).and_then(|c| c.last_mut()) {
        block["cache_control"] = ephemeral();
    }
    let tools: Vec<Value> = body["tools"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|t| json!({ "name": t["function"]["name"], "description": t["function"]["description"], "input_schema": t["function"]["parameters"] }))
        .collect();
    let mut out = json!({ "model": model, "max_tokens": MAX_TOKENS, "messages": messages });
    if !system.is_empty() {
        out["system"] = json!(system);
    }
    if !tools.is_empty() {
        out["tools"] = json!(tools);
        out["tool_choice"] = json!({ "type": "auto" });
    }
    out
}

/// La réponse Messages au format OpenAI que lit l'agent : le texte, les appels d'outils, la réponse native
/// (pour l'échange suivant de la même demande) et ce qu'elle a coûté.
pub fn response(r: &Value) -> Value {
    let blocks = r["content"].as_array().cloned().unwrap_or_default();
    let mut text: String = blocks.iter().filter(|b| b["type"] == "text").filter_map(|b| b["text"].as_str()).collect::<Vec<_>>().join("\n");
    if r["stop_reason"] == "refusal" && text.trim().is_empty() {
        text = "Je ne peux pas répondre à cette demande.".into();
    }
    let calls: Vec<Value> = blocks
        .iter()
        .filter(|b| b["type"] == "tool_use")
        .map(|b| json!({ "id": b["id"], "type": "function", "function": { "name": b["name"], "arguments": b["input"].to_string() } }))
        .collect();
    let mut message = json!({ "role": "assistant", "content": text });
    if !calls.is_empty() {
        message["tool_calls"] = json!(calls);
    }
    message[NATIVE] = json!(blocks);
    let u = &r["usage"];
    let (fresh, read, written) = (u["input_tokens"].as_u64(), u["cache_read_input_tokens"].as_u64(), u["cache_creation_input_tokens"].as_u64());
    // Les tokens envoyés sont la somme des trois : ceux lus en cache et ceux écrits en cache en font partie.
    let prompt = fresh.map(|f| f + read.unwrap_or(0) + written.unwrap_or(0));
    json!({
        "choices": [{ "message": message }],
        "usage": {
            "prompt_tokens": prompt,
            "completion_tokens": u["output_tokens"],
            "prompt_tokens_details": { "cached_tokens": read.unwrap_or(0), "cache_write_tokens": written.unwrap_or(0) },
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn openai_body() -> Value {
        json!({
            "messages": [
                { "role": "system", "content": "Consigne." },
                { "role": "user", "content": "[Page affichée : 0] Mets le titre en rouge." },
                { "role": "assistant", "content": "", "tool_calls": [
                    { "id": "toolu_1", "type": "function", "function": { "name": "get_layout", "arguments": "{}" } },
                    { "id": "toolu_2", "type": "function", "function": { "name": "set_text_style", "arguments": "{\"element_id\":\"t\",\"color\":\"#ff0000\"}" } },
                ] },
                { "role": "tool", "tool_call_id": "toolu_1", "content": "{\"pages\":[]}" },
                { "role": "tool", "tool_call_id": "toolu_2", "content": "{\"ok\":true}" },
            ],
            "tools": [{ "type": "function", "function": { "name": "get_layout", "description": "Mise en page.", "parameters": { "type": "object", "properties": {} } } }],
            "tool_choice": "auto",
        })
    }

    #[test]
    fn only_the_claude_address_takes_the_messages_api() {
        assert!(is_anthropic("https://api.anthropic.com/v1"));
        assert!(is_anthropic(" https://API.anthropic.com/v1/ "));
        for other in ["https://api.openai.com/v1", "http://api.anthropic.com/v1", "https://api.anthropic.com.exemple.fr/v1", "http://localhost:11434/v1"] {
            assert!(!is_anthropic(other), "{other}");
        }
        assert_eq!(endpoint("https://api.anthropic.com/v1"), "https://api.anthropic.com/v1/messages");
        assert_eq!(endpoint("https://api.anthropic.com/v1/"), "https://api.anthropic.com/v1/messages");
        assert_eq!(endpoint("https://api.anthropic.com"), "https://api.anthropic.com/v1/messages");
    }

    #[test]
    fn a_request_becomes_a_messages_request_with_two_cache_marks() {
        let r = request(&openai_body(), "claude-haiku-5-5");
        assert_eq!(r["model"], "claude-haiku-5-5");
        assert_eq!(r["max_tokens"], 16_000);
        assert!(r.get("temperature").is_none());
        assert_eq!(r["system"], json!([{ "type": "text", "text": "Consigne.", "cache_control": { "type": "ephemeral" } }]));
        assert_eq!(r["tools"], json!([{ "name": "get_layout", "description": "Mise en page.", "input_schema": { "type": "object", "properties": {} } }]));
        assert_eq!(r["tool_choice"], json!({ "type": "auto" }));
        let m = r["messages"].as_array().unwrap();
        assert_eq!(m.len(), 3, "user, assistant, puis un seul message de résultats");
        assert_eq!(m[0]["content"][0]["text"], "[Page affichée : 0] Mets le titre en rouge.");
        // Un texte vide n'est pas envoyé : l'API refuse les blocs de texte vides.
        assert_eq!(m[1]["content"].as_array().unwrap().len(), 2);
        assert_eq!(m[1]["content"][1], json!({ "type": "tool_use", "id": "toolu_2", "name": "set_text_style", "input": { "element_id": "t", "color": "#ff0000" } }));
        assert_eq!(m[2]["content"][0], json!({ "type": "tool_result", "tool_use_id": "toolu_1", "content": "{\"pages\":[]}" }));
        assert_eq!(m[2]["content"][1]["tool_use_id"], "toolu_2");
        assert_eq!(m[2]["content"][1]["cache_control"], json!({ "type": "ephemeral" }));
        let marks = r.to_string().matches("cache_control").count();
        assert_eq!(marks, 2);
    }

    #[test]
    fn the_start_of_the_request_is_the_same_from_one_exchange_to_the_next() {
        // Le cache relit un préfixe identique à l'octet : outils et consigne ne bougent pas, les messages
        // s'ajoutent à la fin.
        let first = request(&json!({ "messages": openai_body()["messages"].as_array().unwrap()[..2], "tools": openai_body()["tools"] }), "m");
        let second = request(&openai_body(), "m");
        assert_eq!(first["system"], second["system"]);
        assert_eq!(first["tools"], second["tools"]);
        assert_eq!(first["messages"][0]["content"][0]["text"], second["messages"][0]["content"][0]["text"]);
    }

    #[test]
    fn a_request_without_instructions_has_no_system() {
        // L'essai de connexion (Préférences) n'envoie qu'un message.
        let r = request(&json!({ "messages": [{ "role": "user", "content": "Appelez l'outil ping." }] }), "m");
        assert!(r.get("system").is_none());
        assert!(r.get("tools").is_none());
        assert_eq!(r["messages"][0]["content"][0]["cache_control"], json!({ "type": "ephemeral" }));
    }

    #[test]
    fn the_native_answer_is_sent_back_as_it_came() {
        let native = json!([{ "type": "thinking", "thinking": "", "signature": "abc" }, { "type": "tool_use", "id": "toolu_1", "name": "get_layout", "input": {} }]);
        let mut body = openai_body();
        body["messages"][2][NATIVE] = native.clone();
        let r = request(&body, "m");
        assert_eq!(r["messages"][1]["content"], native);
    }

    #[test]
    fn a_response_becomes_the_message_the_agent_reads() {
        let r = response(&json!({
            "content": [
                { "type": "thinking", "thinking": "", "signature": "abc" },
                { "type": "text", "text": "Je regarde la page." },
                { "type": "tool_use", "id": "toolu_9", "name": "get_layout", "input": { "page": 0 } },
            ],
            "stop_reason": "tool_use",
            "usage": { "input_tokens": 40, "cache_read_input_tokens": 8000, "cache_creation_input_tokens": 600, "output_tokens": 55 },
        }));
        let m = &r["choices"][0]["message"];
        assert_eq!(m["content"], "Je regarde la page.");
        assert_eq!(m["tool_calls"][0], json!({ "id": "toolu_9", "type": "function", "function": { "name": "get_layout", "arguments": "{\"page\":0}" } }));
        assert_eq!(m[NATIVE].as_array().unwrap().len(), 3);
        assert_eq!(r["usage"], json!({ "prompt_tokens": 8640, "completion_tokens": 55, "prompt_tokens_details": { "cached_tokens": 8000, "cache_write_tokens": 600 } }));
    }

    #[test]
    fn a_refusal_without_text_still_answers() {
        let r = response(&json!({ "content": [], "stop_reason": "refusal", "usage": { "input_tokens": 10, "output_tokens": 0 } }));
        assert_eq!(r["choices"][0]["message"]["content"], "Je ne peux pas répondre à cette demande.");
        assert!(r["choices"][0]["message"].get("tool_calls").is_none());
    }
}
