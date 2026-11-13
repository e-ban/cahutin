// Ce que le banc des images attend de la page et du système : le résultat d'une scène, la mémoire de pointe.
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

/**
 * Joue `body` dans la page et rend sa valeur. L'action est lancée, puis son résultat relevé tous les `pasMs` :
 * une seule requête tenue pendant tout un import de milliers de photos dépasserait le délai de réponse de
 * fetch (5 minutes). Échoue après `delaiMs`, ou si la page a été rechargée (la sonde a disparu).
 */
export async function attendreResultat(s, body, args = [], { delaiMs = 15 * 60 * 1000, pasMs = 2000 } = {}) {
  await s.js(
    `window.__resultat = undefined; const args = arguments;
    (async () => { ${body} }).apply(null, args).then((v) => (window.__resultat = { v: v ?? null }), (e) => (window.__resultat = { v: { erreur: String(e) } }));`,
    args,
  );
  const fin = Date.now() + delaiMs;
  for (;;) {
    const { page, r } = await s.js("return { page: typeof window.__mesure === 'function', r: window.__resultat ?? null };");
    if (r) return r.v;
    if (!page) throw new Error("la page a été rechargée pendant la scène");
    if (Date.now() >= fin) throw new Error(`délai de ${Math.round(delaiMs / 1000)} s dépassé`);
    await new Promise((ok) => setTimeout(ok, pasMs));
  }
}

/**
 * Mémoire de pointe, en Mo, de tout le groupe (cgroup) du processus : l'application et ses processus web, qui
 * dessinent la page et décodent les vignettes. Cumulée depuis le lancement. null si on ne la trouve pas.
 */
export function memoirePointeMo(pid, racine = "/") {
  const cgroup = join(racine, "proc", String(pid), "cgroup");
  if (!pid || !existsSync(cgroup)) return null;
  const chemin = readFileSync(cgroup, "utf8").match(/^0::(.*)$/m)?.[1];
  const pic = chemin && join(racine, "sys/fs/cgroup", chemin, "memory.peak");
  if (!pic || !existsSync(pic)) return null;
  return Math.round(Number(readFileSync(pic, "utf8").trim()) / (1024 * 1024));
}

/** Les vignettes du cache d'une photothèque : celles qui sont finies (`.jpg`) et celles en cours d'écriture (`.tmp`). */
export function vignettesFaites(dossier) {
  if (!existsSync(dossier)) return { finies: 0, encours: 0 };
  const noms = readdirSync(dossier);
  return { finies: noms.filter((n) => n.endsWith(".jpg")).length, encours: noms.filter((n) => n.endsWith(".tmp")).length };
}

/**
 * Attend que le fil de fond ait rempli le cache de vignettes : `attendues` vignettes finies, aucune en cours
 * d'écriture, et un compte qui ne bouge plus depuis `calmeMs` (un fil qui reprendrait après un parcours serait vu).
 * Rend le temps écoulé en ms, sans le calme. Échoue après `delaiMs`.
 */
export async function attendreCache(dossier, attendues, { delaiMs = 3 * 60 * 60 * 1000, pasMs = 1000, calmeMs = 5000 } = {}) {
  const debut = Date.now();
  let dernier = -1;
  let depuis = Date.now();
  for (;;) {
    const { finies, encours } = vignettesFaites(dossier);
    if (finies !== dernier) {
      dernier = finies;
      depuis = Date.now();
    }
    if (finies >= attendues && encours === 0 && Date.now() - depuis >= calmeMs) return Date.now() - debut - calmeMs;
    if (Date.now() - debut >= delaiMs) throw new Error(`cache de vignettes incomplet après ${Math.round(delaiMs / 1000)} s (${finies} sur ${attendues})`);
    await new Promise((ok) => setTimeout(ok, pasMs));
  }
}
