//! `cahutin-mcp <document.carnet>` : serveur MCP sur l'entrée et la sortie standard.

use cahutin_mcp::server::Server;
use std::io::{BufRead, Write};

fn main() {
    let Some(path) = std::env::args_os().nth(1) else {
        eprintln!("usage : cahutin-mcp <document.carnet>");
        std::process::exit(2);
    };
    let server = Server { path: path.into() };
    let mut out = std::io::stdout().lock();
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str(&line) {
            Ok(msg) => server.handle(&msg),
            Err(_) => Some(serde_json::json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": "JSON illisible" } })),
        };
        if let Some(r) = reply {
            let _ = writeln!(out, "{r}");
            let _ = out.flush();
        }
    }
}
