// Petit client WebDriver pour tauri-driver : une session sur l'application de développement, exécuter du JS,
// redimensionner, capturer.
import { writeFileSync } from "node:fs";

const BASE = "http://localhost:4444";
async function call(method, path, body) {
  const r = await fetch(BASE + path, { method, headers: { "content-type": "application/json" }, body: body ? JSON.stringify(body) : undefined });
  const j = await r.json();
  if (j.value && j.value.error) throw new Error(`${path}: ${j.value.error} ${j.value.message}`);
  return j.value;
}

export async function session(app) {
  const v = await call("POST", "/session", { capabilities: { alwaysMatch: { "tauri:options": { application: app } } } });
  const id = v.sessionId;
  const s = `/session/${id}`;
  return {
    id,
    js: (script, args = []) => call("POST", `${s}/execute/sync`, { script, args }),
    jsAsync: (script, args = []) => call("POST", `${s}/execute/async`, { script, args }),
    size: (width, height) => call("POST", `${s}/window/rect`, { width, height }),
    shot: async (file) => writeFileSync(file, Buffer.from(await call("GET", `${s}/screenshot`), "base64")),
    click: async (css) => {
      const el = await call("POST", `${s}/element`, { using: "css selector", value: css });
      const key = Object.keys(el)[0];
      return call("POST", `${s}/element/${el[key]}/click`, {});
    },
    end: () => call("DELETE", s),
  };
}

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
