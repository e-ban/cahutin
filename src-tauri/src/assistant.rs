//! Assistant de mise en page : réglages (désactivé par défaut), client du modèle (API « compatible
//! OpenAI ») et commandes. La boucle et les outils sont dans `cahutin_mcp::agent`.

use crate::state::AppState;
use cahutin_mcp::agent::{self, ChatError, Event, Request};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use ts_rs::TS;

const FILE: &str = "assistant.json";
/// Délai d'une requête au modèle : un petit modèle sur un processeur peut être long.
const TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub enabled: bool,
    pub url: String,
    pub model: String,
    pub api_key: String,
}

/// Les réglages vus par l'interface : jamais la clé, seulement s'il y en a une.
#[derive(Serialize, Clone, Debug, TS)]
#[ts(export)]
pub struct AssistantSettings {
    pub enabled: bool,
    pub url: String,
    pub model: String,
    pub has_key: bool,
}

pub fn load(dir: &Path) -> Settings {
    std::fs::read_to_string(dir.join(FILE)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn view(s: &Settings) -> AssistantSettings {
    AssistantSettings { enabled: s.enabled, url: s.url.clone(), model: s.model.clone(), has_key: !s.api_key.is_empty() }
}

/// `api_key` : `None` garde la clé enregistrée (pour la même adresse seulement), `Some("")` l'efface.
pub fn configure(dir: &Path, enabled: bool, url: String, model: String, api_key: Option<String>) -> Result<AssistantSettings, String> {
    let mut s = load(dir);
    // Une autre adresse ne reçoit jamais la clé enregistrée pour la précédente : il faut la taper.
    if api_key.is_none() && s.url.trim_end_matches('/') != url.trim().trim_end_matches('/') {
        s.api_key.clear();
    }
    s.enabled = enabled;
    s.url = url.trim().to_string();
    s.model = model.trim().to_string();
    if let Some(k) = api_key {
        s.api_key = k.trim().to_string();
    }
    std::fs::create_dir_all(dir).map_err(|_| "Les réglages ne peuvent pas être enregistrés.".to_string())?;
    let text = serde_json::to_string_pretty(&s).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(FILE), text).map_err(|_| "Les réglages ne peuvent pas être enregistrés.".to_string())?;
    Ok(view(&s))
}

pub fn endpoint(url: &str) -> String {
    format!("{}/chat/completions", url.trim_end_matches('/'))
}

pub fn chat_error(status: u16, _body: &str) -> ChatError {
    match status {
        401 | 403 => ChatError::KeyRefused,
        404 => ChatError::ModelUnknown,
        429 => ChatError::Busy,
        500.. => ChatError::ServiceDown,
        // Le corps de la réponse (en anglais, souvent du JSON) ne s'affiche ni ne s'écrit au journal : il peut
        // reprendre la demande.
        _ => {
            log::warn!("assistant_http_error status={status}");
            ChatError::Other(format!("code {status}"))
        }
    }
}

/// Une requête au modèle ; rend le message du premier choix.
fn chat(s: &Settings, body: &Value) -> Result<Value, ChatError> {
    use ureq::tls::{RootCerts, TlsConfig};
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .http_status_as_error(false)
        .user_agent("Cahutin")
        .tls_config(TlsConfig::builder().root_certs(RootCerts::PlatformVerifier).build())
        .build()
        .into();
    let mut body = body.clone();
    body["model"] = json!(s.model);
    let mut req = agent.post(endpoint(&s.url)).header("Content-Type", "application/json");
    if !s.api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", s.api_key));
    }
    let mut response = req.send(body.to_string()).map_err(|e| if matches!(e, ureq::Error::Timeout(_)) { ChatError::TimedOut } else { ChatError::Unreachable })?;
    let status = response.status().as_u16();
    let text = response.body_mut().with_config().limit(20_000_000).read_to_string().map_err(|e| if matches!(e, ureq::Error::Timeout(_)) { ChatError::TimedOut } else { ChatError::Unreachable })?;
    if status != 200 {
        return Err(chat_error(status, &text));
    }
    let v: Value = serde_json::from_str(&text).map_err(|_| ChatError::Other("réponse illisible".into()))?;
    v["choices"][0].get("message").cloned().ok_or(ChatError::Other("réponse sans message".into()))
}

/// Fait `f` dans un autre fil et rend la main dès que `cancel` passe à vrai, sans attendre la réponse du
/// modèle (qui peut prendre des minutes sur un processeur) ; la requête abandonnée finit seule.
pub fn cancellable(f: impl FnOnce() -> Result<Value, ChatError> + Send + 'static, cancel: &AtomicBool) -> Result<Value, ChatError> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(r) => return r,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return Err(ChatError::Other("requête interrompue".into())),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) if cancel.load(Ordering::SeqCst) => return Err(ChatError::Cancelled),
            Err(_) => {}
        }
    }
}

/// Vérifie que le service répond et que le modèle sait appeler un outil.
pub fn test_connection(s: &Settings) -> Result<(), String> {
    let ping = json!([{ "type": "function", "function": { "name": "ping", "description": "Répond pong.", "parameters": { "type": "object", "properties": {} } } }]);
    let body = json!({ "messages": [{ "role": "user", "content": "Appelez l'outil ping." }], "tools": ping, "tool_choice": "auto" });
    let message = chat(s, &body).map_err(|e| e.message_fr())?;
    let called = message["tool_calls"].as_array().is_some_and(|c| c.iter().any(|tc| tc["function"]["name"] == "ping"));
    if called { Ok(()) } else { Err("Ce modèle ne sait pas utiliser les outils de Cahutin. Choisissez-en un autre.".into()) }
}

/// Clé de l'essai de connexion : celle tapée, sinon la clé enregistrée, mais seulement vers l'adresse
/// enregistrée (une clé ne part jamais vers une autre adresse qu'on vient de taper).
pub fn key_for(saved: &Settings, url: &str, typed: Option<String>) -> String {
    match typed {
        Some(k) if !k.is_empty() => k,
        _ if url.trim() == saved.url => saved.api_key.clone(),
        _ => String::new(),
    }
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_config_dir().map_err(|_| "Le dossier des réglages est introuvable.".to_string())
}

#[tauri::command]
pub fn assistant_settings(app: AppHandle) -> Result<AssistantSettings, String> {
    Ok(view(&load(&config_dir(&app)?)))
}

#[tauri::command]
pub fn assistant_configure(app: AppHandle, enabled: bool, url: String, model: String, api_key: Option<String>) -> Result<AssistantSettings, String> {
    configure(&config_dir(&app)?, enabled, url, model, api_key)
}

/// Exécutée hors du fil principal : la fenêtre reste réactive.
#[tauri::command(async)]
pub fn assistant_test(app: AppHandle, url: String, model: String, api_key: Option<String>) -> Result<(), String> {
    let saved = load(&config_dir(&app)?);
    let api_key = key_for(&saved, &url, api_key);
    let s = Settings { enabled: true, url, model, api_key };
    test_connection(&s)
}

/// Exécutée hors du fil principal : chaque étape part vers l'interface dès qu'elle est faite.
#[tauri::command(async)]
pub fn assistant_send(app: AppHandle, state: State<'_, AppState>, request: Request, on_event: Channel<Event>) -> Result<(), String> {
    let s = load(&config_dir(&app)?);
    if !s.enabled {
        return Err("L'assistant est désactivé.".into());
    }
    state.assistant_cancel.store(false, Ordering::SeqCst);
    log::info!("assistant_request");
    let cancel = &state.assistant_cancel;
    agent::run(
        request,
        &mut |body| {
            let (s, body) = (s.clone(), body.clone());
            cancellable(move || chat(&s, &body), cancel)
        },
        &mut |e| {
            let _ = on_event.send(e);
        },
        cancel,
    );
    Ok(())
}

#[tauri::command]
pub fn assistant_stop(state: State<'_, AppState>) {
    state.assistant_cancel.store(true, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_are_off_and_empty_by_default() {
        let dir = tempfile::tempdir().unwrap();
        let s = load(dir.path());
        assert!(!s.enabled);
        assert_eq!((s.url.as_str(), s.model.as_str(), s.api_key.as_str()), ("", "", ""));
    }

    #[test]
    fn settings_roundtrip_and_never_show_the_key() {
        let dir = tempfile::tempdir().unwrap();
        let v = configure(dir.path(), true, "http://localhost:11434/v1".into(), "qwen3:4b".into(), Some("secret".into())).unwrap();
        assert!(v.enabled && v.has_key);
        assert!(!serde_json::to_string(&v).unwrap().contains("secret"));
        assert_eq!(load(dir.path()).api_key, "secret");
        // Sans clé donnée, la clé reste ; une clé vide l'efface.
        configure(dir.path(), false, "http://localhost:11434/v1".into(), "qwen3:4b".into(), None).unwrap();
        assert_eq!(load(dir.path()).api_key, "secret");
        let v = configure(dir.path(), true, "http://x/v1".into(), "m".into(), Some(String::new())).unwrap();
        assert!(!v.has_key);
        assert_eq!(view(&load(dir.path())).url, "http://x/v1");
    }

    #[test]
    fn http_statuses_become_plain_errors() {
        assert_eq!(chat_error(401, ""), ChatError::KeyRefused);
        assert_eq!(chat_error(403, ""), ChatError::KeyRefused);
        assert_eq!(chat_error(404, "model not found"), ChatError::ModelUnknown);
        // Ni anglais ni JSON du service à l'écran : un message clair, le code seulement s'il n'y en a pas.
        assert_eq!(chat_error(429, "{\"error\":\"rate limit\"}"), ChatError::Busy);
        assert_eq!(chat_error(500, "boom"), ChatError::ServiceDown);
        assert_eq!(chat_error(503, "boom"), ChatError::ServiceDown);
        assert_eq!(chat_error(418, "teapot"), ChatError::Other("code 418".into()));
        for e in [ChatError::Busy, ChatError::ServiceDown, ChatError::TimedOut, chat_error(418, "teapot")] {
            assert!(!e.message_fr().contains("teapot"), "{e:?}");
        }
        assert!(ChatError::TimedOut.message_fr().contains("trop de temps"));
    }

    #[test]
    fn a_new_address_never_receives_the_saved_key() {
        let dir = tempfile::tempdir().unwrap();
        configure(dir.path(), true, "https://payant/v1".into(), "m".into(), Some("secret".into())).unwrap();
        configure(dir.path(), true, "https://payant/v1/".into(), "m2".into(), None).unwrap();
        assert_eq!(load(dir.path()).api_key, "secret", "même adresse : la clé reste");
        let v = configure(dir.path(), true, "https://autre/v1".into(), "m".into(), None).unwrap();
        assert!(!v.has_key, "autre adresse sans clé tapée : la clé est effacée");
    }

    #[test]
    fn stop_does_not_wait_for_the_model() {
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let c = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            c.store(true, Ordering::SeqCst);
        });
        let start = std::time::Instant::now();
        let r = cancellable(
            || {
                std::thread::sleep(Duration::from_secs(5));
                Ok(json!({}))
            },
            &cancel,
        );
        assert_eq!(r, Err(ChatError::Cancelled));
        assert!(start.elapsed() < Duration::from_secs(1));
        assert_eq!(cancellable(|| Ok(json!({ "ok": true })), &std::sync::atomic::AtomicBool::new(false)), Ok(json!({ "ok": true })));
    }

    #[test]
    fn the_saved_key_only_goes_to_the_saved_address() {
        let saved = Settings { enabled: true, url: "http://a/v1".into(), model: "m".into(), api_key: "secret".into() };
        assert_eq!(key_for(&saved, "http://a/v1", None), "secret");
        assert_eq!(key_for(&saved, "http://autre/v1", None), "");
        assert_eq!(key_for(&saved, "http://autre/v1", Some("k2".into())), "k2");
    }

    #[test]
    fn the_endpoint_is_built_from_the_address() {
        assert_eq!(endpoint("http://localhost:11434/v1"), "http://localhost:11434/v1/chat/completions");
        assert_eq!(endpoint("http://localhost:11434/v1/"), "http://localhost:11434/v1/chat/completions");
    }

    /// Avec Ollama lancé (`OLLAMA_CONTEXT_LENGTH=16384 ollama serve`) et `qwen3:4b` téléchargé.
    #[test]
    #[ignore]
    fn a_local_model_can_call_tools() {
        let s = Settings { enabled: true, url: "http://localhost:11434/v1".into(), model: "qwen3:4b".into(), api_key: String::new() };
        test_connection(&s).unwrap();
    }
}
