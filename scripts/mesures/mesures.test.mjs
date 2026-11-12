// Tests du banc des images : node --test scripts/mesures/
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { attendreResultat, memoirePointeMo } from "./attente.mjs";
import { SONDE } from "./sonde.mjs";

/** Une fausse page : des images d'animation toutes les 16 ms, et les vignettes données. */
function fausse(imgs = []) {
  globalThis.window = globalThis;
  globalThis.requestAnimationFrame = (f) => setTimeout(() => f(performance.now()), 16);
  globalThis.document = { querySelectorAll: () => imgs };
  globalThis.innerHeight = 800;
  globalThis.innerWidth = 800;
  (0, eval)(SONDE);
}
const vignette = (complete = true) => ({ complete, naturalWidth: complete ? 10 : 0, dataset: {}, getBoundingClientRect: () => ({ width: 10, top: 10, bottom: 20, left: 10, right: 20 }) });

test("une action en échec rend son erreur tout de suite", async () => {
  fausse();
  const debut = Date.now();
  const r = await window.__mesure(() => Promise.reject(new Error("import impossible")), () => false, 5000);
  assert.match(r.erreur, /import impossible/);
  assert.ok(Date.now() - debut < 1000);
});

test("une scène jamais prête expire", async () => {
  fausse();
  const r = await window.__mesure(() => {}, () => false, 300);
  assert.equal(r.expire, true);
});

test("les vignettes déjà chargées avant l'action ne comptent pas", () => {
  const avant = [vignette(), vignette(), vignette()];
  const apres = vignette(false);
  fausse([...avant, apres]);
  window.__marquerChargees(".x");
  assert.equal(window.__vignettesPretes(".x", 3), false);
  apres.complete = true;
  apres.naturalWidth = 10;
  assert.equal(window.__vignettesPretes(".x", 1), true);
  assert.equal(window.__vignettesPretes(".x", 2), false);
});

test("attendreResultat abandonne après son délai", async () => {
  const s = { js: async (script) => (script.includes("__resultat ??") ? { page: true, r: null } : null) };
  await assert.rejects(attendreResultat(s, "await new Promise(() => {})", [], { delaiMs: 200, pasMs: 20 }), /délai/);
});

test("attendreResultat s'arrête si la page a été rechargée", async () => {
  const s = { js: async (script) => (script.includes("__resultat ??") ? { page: false, r: null } : null) };
  await assert.rejects(attendreResultat(s, "", [], { delaiMs: 5000, pasMs: 20 }), /rechargée/);
});

test("attendreResultat rend la valeur de la scène", async () => {
  const s = { js: async (script) => (script.includes("__resultat ??") ? { page: true, r: { v: 42 } } : null) };
  assert.equal(await attendreResultat(s, "", [], { delaiMs: 5000, pasMs: 20 }), 42);
});

test("la mémoire de pointe est celle de tout le groupe de l'application", () => {
  const racine = mkdtempSync(join(tmpdir(), "mesures-"));
  mkdirSync(join(racine, "proc/42"), { recursive: true });
  writeFileSync(join(racine, "proc/42/cgroup"), "0::/user.slice/run-r1.scope\n");
  mkdirSync(join(racine, "sys/fs/cgroup/user.slice/run-r1.scope"), { recursive: true });
  writeFileSync(join(racine, "sys/fs/cgroup/user.slice/run-r1.scope/memory.peak"), String(1536 * 1024 * 1024) + "\n");
  assert.equal(memoirePointeMo("42", racine), 1536);
  assert.equal(memoirePointeMo("7", racine), null);
});
