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
    if enabled {
        check_url(&url)?;
    }
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

/// L'hôte d'une adresse `http(s)://hôte[:port]/…`, en minuscules ; rien pour une autre adresse.
fn host_of(url: &str) -> Option<(bool, String)> {
    let url = url.trim().to_lowercase();
    let (secure, rest) = if let Some(r) = url.strip_prefix("https://") { (true, r.to_string()) } else { (false, url.strip_prefix("http://")?.to_string()) };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    // Un nom d'utilisateur (`hôte@…`) cacherait le vrai hôte : une telle adresse n'est jamais acceptée.
    if authority.contains('@') {
        return None;
    }
    let host = if authority.starts_with('[') { authority.split(']').next().map(|h| format!("{h}]")).unwrap_or_default() } else { authority.split(':').next().unwrap_or("").to_string() };
    (!host.is_empty()).then_some((secure, host))
}

/// Ce qui sort de l'ordinateur part chiffré : `https://`, ou `http://` vers cet ordinateur seulement.
pub fn check_url(url: &str) -> Result<(), String> {
    match host_of(url) {
        Some((true, _)) => Ok(()),
        Some((false, h)) if ["localhost", "127.0.0.1", "[::1]"].contains(&h.as_str()) => Ok(()),
        _ => Err("Cette adresse n'est pas chiffrée : seule une adresse qui commence par https:// est acceptée, sauf pour un service sur cet ordinateur (localhost).".into()),
    }
}

/// Le signalement d'une réponse, à envoyer à l'auteur : la demande, la réponse et ses étapes, le service
/// (son seul nom d'hôte) et le modèle ; jamais la clé.
pub fn report_text(version: &str, date: &str, url: &str, model: &str, request: &str, reply: &str, steps: &[String]) -> String {
    let host = host_of(url).map(|(_, h)| h).unwrap_or_default();
    let steps: String = steps.iter().map(|s| format!("- {s}\n")).collect();
    format!(
        "Signalement d'une réponse de l'assistant de Cahutin\n\nCahutin {version}, le {date}\nService : {host}, modèle {model}\n\nDemande :\n{request}\n\nRéponse :\n{reply}\n\nÉtapes :\n{steps}"
    )
}

/// Où arrivent les signalements : l'adresse dédiée, la même que pour « Signaler un problème ».
pub const REPORT_EMAIL: &str = carnet_core::report::REPORT_EMAIL;
/// Longueur d'un lien `mailto:` que toutes les messageries acceptent.
const MAILTO_MAX: usize = 1800;

/// Encodage d'un lien : seuls les caractères sans réserve restent tels quels ; un retour à la ligne devient CRLF.
fn encode(text: &str, room: usize) -> (String, bool) {
    let mut out = String::new();
    for c in text.replace("\r\n", "\n").chars() {
        let piece: String = if c.is_ascii_alphanumeric() || "-_.~".contains(c) {
            c.to_string()
        } else if c == '\n' {
            "%0D%0A".into()
        } else {
            let mut buf = [0u8; 4];
            c.encode_utf8(&mut buf).bytes().map(|b| format!("%{b:02X}")).collect()
        };
        if out.len() + piece.len() > room {
            return (out, true);
        }
        out.push_str(&piece);
    }
    (out, false)
}

/// Un e-mail prêt à envoyer à l'auteur, avec le signalement dedans (coupé s'il est trop long).
pub fn report_mailto(text: &str) -> String {
    let head = format!("mailto:{REPORT_EMAIL}?subject={}&body=", encode("Signalement d'une réponse de l'assistant", usize::MAX).0);
    let ellipsis = "%E2%80%A6";
    let (body, cut) = encode(text, MAILTO_MAX - head.len() - ellipsis.len());
    format!("{head}{body}{}", if cut { ellipsis } else { "" })
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

/// Ce qu'a coûté une demande : les échanges avec le service, les appels d'outils, et les tokens envoyés,
/// reçus et lus en cache. Un compte de tokens est inconnu (`None`) dès qu'un échange ne l'a pas donné :
/// mieux vaut « ? » qu'un total trop bas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Usage {
    pub rounds: u32,
    pub tool_calls: u32,
    pub prompt: Option<u64>,
    pub completion: Option<u64>,
    pub cached: Option<u64>,
}

impl Default for Usage {
    fn default() -> Self {
        Usage { rounds: 0, tool_calls: 0, prompt: Some(0), completion: Some(0), cached: Some(0) }
    }
}

impl Usage {
    /// Ajoute un échange : la réponse entière du service (format OpenAI, champ `usage`).
    pub fn add(&mut self, response: &Value) {
        fn sum(total: &mut Option<u64>, n: Option<u64>) {
            *total = total.zip(n).map(|(a, b)| a + b);
        }
        let u = &response["usage"];
        self.rounds += 1;
        self.tool_calls += response["choices"][0]["message"]["tool_calls"].as_array().map_or(0, Vec::len) as u32;
        sum(&mut self.prompt, u["prompt_tokens"].as_u64());
        sum(&mut self.completion, u["completion_tokens"].as_u64());
        sum(&mut self.cached, u["prompt_tokens_details"]["cached_tokens"].as_u64());
    }

    /// La ligne du journal : des nombres seulement, jamais la demande ni la réponse.
    pub fn line(&self) -> String {
        let n = |v: Option<u64>| v.map_or("?".to_string(), |v| v.to_string());
        format!(
            "assistant_usage rounds={} tool_calls={} prompt={} completion={} cached={}",
            self.rounds,
            self.tool_calls,
            n(self.prompt),
            n(self.completion),
            n(self.cached)
        )
    }
}

/// Le message du premier choix d'une réponse du service.
fn message_of(response: &Value) -> Result<Value, ChatError> {
    response["choices"][0].get("message").cloned().ok_or(ChatError::Other("réponse sans message".into()))
}

/// Une requête au modèle ; rend la réponse entière (le message et ce qu'il a coûté).
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
    serde_json::from_str(&text).map_err(|_| ChatError::Other("réponse illisible".into()))
}

/// Fait la demande avec le service réglé, en relevant ce qu'elle a coûté.
pub fn run_measured(s: &Settings, request: Request, on_event: &mut dyn FnMut(Event), cancel: &AtomicBool) -> Usage {
    let mut usage = Usage::default();
    agent::run(
        request,
        &mut |body| {
            let (s, body) = (s.clone(), body.clone());
            let response = cancellable(move || chat(&s, &body), cancel)?;
            usage.add(&response);
            message_of(&response)
        },
        on_event,
        cancel,
    );
    usage
}

/// Fait `f` dans un autre fil et rend la main dès que `cancel` passe à vrai, sans attendre la réponse du
/// modèle (qui peut prendre des minutes sur un processeur) ; la requête abandonnée finit seule.
pub fn cancellable<T: Send + 'static>(f: impl FnOnce() -> Result<T, ChatError> + Send + 'static, cancel: &AtomicBool) -> Result<T, ChatError> {
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
    let message = chat(s, &body).and_then(|r| message_of(&r)).map_err(|e| e.message_fr())?;
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
    check_url(&url)?;
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
    check_url(&s.url)?;
    state.assistant_cancel.store(false, Ordering::SeqCst);
    log::info!("assistant_request");
    let usage = run_measured(
        &s,
        request,
        &mut |e| {
            let _ = on_event.send(e);
        },
        &state.assistant_cancel,
    );
    log::info!("{}", usage.line());
    Ok(())
}

/// Ouvre la messagerie sur un e-mail à l'auteur avec le signalement d'une réponse, à relire puis envoyer.
#[tauri::command]
pub fn assistant_report(app: AppHandle, request: String, reply: String, steps: Vec<String>) -> Result<String, String> {
    let s = load(&config_dir(&app)?);
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let date = now.format(&time::format_description::parse_borrowed::<2>("[year]-[month]-[day] [hour]:[minute]").unwrap()).unwrap_or_default();
    let text = report_text(env!("CARGO_PKG_VERSION"), &date, &s.url, &s.model, &request, &reply, &steps);
    open::that_detached(report_mailto(&text)).map_err(|_| format!("Aucune messagerie ne s'est ouverte : écrivez à {REPORT_EMAIL}."))?;
    Ok(REPORT_EMAIL.to_string())
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
        let v = configure(dir.path(), true, "https://x/v1".into(), "m".into(), Some(String::new())).unwrap();
        assert!(!v.has_key);
        assert_eq!(view(&load(dir.path())).url, "https://x/v1");
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

    fn response(usage: Value, calls: usize) -> Value {
        let tool_calls: Vec<Value> = (0..calls).map(|i| json!({ "id": format!("c{i}"), "type": "function", "function": { "name": "get_layout", "arguments": "{}" } })).collect();
        let message = if calls > 0 { json!({ "role": "assistant", "content": null, "tool_calls": tool_calls }) } else { json!({ "role": "assistant", "content": "Fait." }) };
        json!({ "choices": [{ "message": message }], "usage": usage })
    }

    #[test]
    fn the_cost_of_a_request_adds_up_its_exchanges() {
        let mut u = Usage::default();
        u.add(&response(json!({ "prompt_tokens": 5000, "completion_tokens": 40, "prompt_tokens_details": { "cached_tokens": 0 } }), 2));
        u.add(&response(json!({ "prompt_tokens": 6000, "completion_tokens": 60, "prompt_tokens_details": { "cached_tokens": 4800 } }), 1));
        u.add(&response(json!({ "prompt_tokens": 6500, "completion_tokens": 20, "prompt_tokens_details": { "cached_tokens": 5900 } }), 0));
        assert_eq!(u, Usage { rounds: 3, tool_calls: 3, prompt: Some(17500), completion: Some(120), cached: Some(10700) });
        assert_eq!(u.line(), "assistant_usage rounds=3 tool_calls=3 prompt=17500 completion=120 cached=10700");
    }

    #[test]
    fn an_unreported_count_is_unknown_not_zero() {
        let mut u = Usage::default();
        u.add(&response(json!({ "prompt_tokens": 5000, "completion_tokens": 40 }), 1));
        u.add(&response(Value::Null, 0));
        assert_eq!((u.rounds, u.prompt, u.completion, u.cached), (2, None, None, None));
        assert_eq!(u.line(), "assistant_usage rounds=2 tool_calls=1 prompt=? completion=? cached=?");
        // Une demande refusée avant d'être envoyée (trop longue) n'a rien coûté.
        assert_eq!(Usage::default().line(), "assistant_usage rounds=0 tool_calls=0 prompt=0 completion=0 cached=0");
    }

    #[test]
    fn the_usage_line_never_holds_the_request() {
        // Même si le service renvoie la demande dans sa réponse, seule la ligne chiffrée va au journal.
        let mut u = Usage::default();
        let mut r = response(json!({ "prompt_tokens": 10, "completion_tokens": 2 }), 0);
        r["choices"][0]["message"]["content"] = json!("Mets le prénom de Léa en rouge");
        u.add(&r);
        let line = u.line();
        assert!(!line.contains("Léa") && !line.contains("rouge"), "{line}");
        assert!(line.split(' ').skip(1).all(|kv| kv.split_once('=').is_some_and(|(_, v)| v == "?" || v.parse::<u64>().is_ok())), "{line}");
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

    #[test]
    fn only_an_encrypted_address_leaves_the_computer() {
        for ok in ["https://api.openai.com/v1", "http://localhost:11434/v1", "http://127.0.0.1:8080/v1", "http://[::1]:11434/v1", " HTTP://LOCALHOST/v1 "] {
            assert_eq!(check_url(ok), Ok(()), "{ok}");
        }
        for bad in ["http://192.168.1.20:11434/v1", "http://exemple.fr/v1", "http://localhost.exemple.fr/v1", "ftp://exemple.fr", "exemple.fr/v1", "", "http://localhost:11434@exemple.fr/v1", "http://[::1]@exemple.fr/v1"] {
            assert!(check_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn an_unencrypted_address_is_never_saved_switched_on() {
        let dir = tempfile::tempdir().unwrap();
        assert!(configure(dir.path(), true, "http://exemple.fr/v1".into(), "m".into(), Some("secret".into())).is_err());
        assert_eq!(load(dir.path()).api_key, "");
    }

    #[test]
    fn a_report_names_the_service_but_never_the_key() {
        let text = report_text("0.13.0", "2026-11-07 22:00", "https://api.openai.com/v1", "gpt-6-luna", "Mets un titre", "Voilà un titre.", &["Texte ajouté".into()]);
        for part in ["0.13.0", "2026-11-07 22:00", "api.openai.com", "gpt-6-luna", "Mets un titre", "Voilà un titre.", "Texte ajouté"] {
            assert!(text.contains(part), "{part}");
        }
        assert!(!text.contains("/v1"));
    }

    #[test]
    fn a_report_opens_as_a_ready_email() {
        let url = report_mailto("Demande :\nMets un titre à Léa & co");
        assert!(url.starts_with(&format!("mailto:{REPORT_EMAIL}?subject=")), "{url}");
        assert!(url.contains("&body=Demande%20%3A%0D%0AMets%20un%20titre%20%C3%A0%20L%C3%A9a%20%26%20co"), "{url}");
        assert!(!url.contains(' ') && !url.contains('\n'));
    }

    #[test]
    fn a_long_report_is_cut_to_fit_a_mail_link() {
        let url = report_mailto(&"é".repeat(5000));
        assert!(url.len() <= MAILTO_MAX, "{}", url.len());
        assert!(url.ends_with("%E2%80%A6"), "ends with an ellipsis");
    }
}

#[cfg(test)]
mod bench;
