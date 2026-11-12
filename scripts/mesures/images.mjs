// Banc des images : temps d'affichage du tiroir Photos et de la photothèque, gels de l'interface et mémoire,
// dans l'application de développement pilotée par WebDriver. Photos générées par photos.sh, jamais de vraies.
// Usage : node images.mjs <dossier photos> <dossier sortie> [scène…]
// Écrit <sortie>/mesures.xml (JUnit, pour `timonier test --junit`) et <sortie>/mesures.md.
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { session, sleep } from "../captures/wd.mjs";
import { attendreResultat, memoirePointeMo } from "./attente.mjs";
import { SONDE } from "./sonde.mjs";

const [photosDir, out, ...only] = process.argv.slice(2);
if (!photosDir || !out) throw new Error("Usage : node images.mjs <dossier photos> <dossier sortie> [scène…]");
const APP = new URL("../../target/debug/cahutin", import.meta.url).pathname;
const SEUIL_MS = 1000;
const SEUIL_GEL_MS = 100;
const TIROIR = [100, 300, 500];
// 5 000 et 10 000 viendront avec les dossiers suivis (étape 4) : aujourd'hui, chaque photo est copiée, et
// remplir une photothèque de 10 000 photos prendrait deux heures.
const PHOTOTHEQUE = [1000];
/** Une sortie : le lot qu'on ajoute d'un coup dans la vraie vie. */
const LOT = 50;
const ELEVES = 25;

/** Les photos générées, dans l'ordre (sortie-01/photo-00001.jpg, …). */
function listePhotos(dir) {
  return readdirSync(dir)
    .filter((d) => d.startsWith("sortie-"))
    .sort()
    .flatMap((d) => readdirSync(join(dir, d)).filter((f) => f.endsWith(".jpg")).sort().map((f) => join(dir, d, f)));
}

const photos = listePhotos(photosDir);
const voulue = (scene) => only.length === 0 || only.includes(scene);
const resultats = [];
let s;

/** Joue `body` dans la page et rend sa valeur ; échoue après `delaiMs` (15 minutes par défaut). */
const run = (body, args = [], delaiMs) => attendreResultat(s, body, args, delaiMs ? { delaiMs } : {});

/** Mémoire de pointe de l'application et de ses processus web, cumulée depuis le lancement ; null sans PID. */
function memoireMo() {
  const pid = process.env.CAHUTIN_PID || (existsSync(join(out, ".pid")) ? readFileSync(join(out, ".pid"), "utf8").trim() : "");
  return memoirePointeMo(pid);
}

/**
 * Une mesure : `corps` rend le résultat de `__mesure`. `seuil.ms` borne le temps, `seuil.gel` le plus long gel.
 * Une scène non demandée n'est pas mesurée ; `prealable` la joue quand même (un import dont dépendent les
 * suivantes), sans l'enregistrer.
 */
async function mesurer(scene, taille, corps, seuil, { prealable = false } = {}) {
  if (!voulue(scene) && !prealable) return;
  const nom = `${scene}-${taille}`;
  let r;
  try {
    r = await run(corps);
  } catch (e) {
    r = { erreur: String(e) };
  }
  if (!voulue(scene)) return;
  const res = { nom, scene, taille, memoireMo: memoireMo() };
  if (!r || r.erreur) res.erreur = r?.erreur ?? "aucun résultat";
  else {
    Object.assign(res, r);
    const raisons = [];
    if (r.expire) raisons.push("jamais prêt (délai dépassé)");
    if (seuil.ms != null && r.ms > seuil.ms) raisons.push(`${r.ms} ms > ${seuil.ms} ms`);
    if (seuil.gel != null && r.gelMaxMs > seuil.gel) raisons.push(`gel de ${r.gelMaxMs} ms > ${seuil.gel} ms`);
    if (raisons.length) res.echec = raisons.join(" ; ");
  }
  resultats.push(res);
  console.log(nom, res.erreur ? `ERREUR ${res.erreur}` : `${res.ms} ms, gel ${res.gelMaxMs} ms${res.echec ? `, ÉCHEC ${res.echec}` : ""}`);
}

const attendre = (ms) => `await new Promise((r) => setTimeout(r, ${ms}));`;

async function tiroir() {
  if (!TIROIR.length || !["tiroir-import", "tiroir-ouvrir", "tiroir-filtre", "tiroir-defiler"].some(voulue)) return;
  await run(`
    const e = window.__editor, c = window.__carnet;
    e.showPreferences = false; e.drawer = null; e.selection = [];
    if (c.guides.close) c.guides.close();
    await c.openExampleDoc("porte-manteaux");
    e.showWelcome = false;
    e.replaceLibrary([]);
  `);
  await sleep(1500);
  let deja = 0;
  for (const n of TIROIR) {
    const lot = photos.slice(deja, n);
    await mesurer(
      "tiroir-import",
      n,
      `const e = window.__editor, c = window.__carnet, lot = ${JSON.stringify(lot)};
      e.openImages("photos"); window.__fait = false;
      return await window.__mesure(async () => { await c.importPaths(lot, { quiet: true }); window.__fait = true; },
        () => window.__fait && window.__vignettesPretes(".drawer .body", ${n}), 600000);`,
      { gel: SEUIL_GEL_MS },
      { prealable: true },
    );
    deja = n;
    await mesurer(
      "tiroir-ouvrir",
      n,
      `const e = window.__editor; e.drawer = null; ${attendre(500)}
      return await window.__mesure(() => e.openImages("photos"), () => window.__vignettesPretes(".drawer .body", ${n}));`,
      { ms: SEUIL_MS },
    );
    await mesurer(
      "tiroir-filtre",
      n,
      // Le dossier de la photo affichée tout en bas de la liste. Ses vignettes peuvent être déjà chargées (le
      // tiroir range les photos par jour : pendant l'import, chacune est passée en haut) : prêt quand la liste
      // ne montre plus que ce dossier, et que les vignettes visibles sont chargées. (« Non placées » ne
      // changerait rien ici : aucune photo n'est posée.)
      `const puces = () => [...document.querySelectorAll(".drawer .chips button")];
      const vignettes = () => [...document.querySelectorAll(".drawer .body img[data-asset]")];
      const dossierDe = new Map(window.__editor.photos.map((p) => [p.meta.id, p.folder]));
      const dossier = dossierDe.get(vignettes().at(-1).dataset.asset);
      const puce = puces().find((b) => b.textContent.trim().startsWith(dossier + " ("));
      const r = await window.__mesure(() => puce.click(), () => {
        const v = vignettes();
        return v.length > 0 && v.every((img) => dossierDe.get(img.dataset.asset) === dossier) && window.__vignettesPretes(".drawer .body", v.length);
      });
      puces()[0].click(); return r;`,
      { ms: SEUIL_MS },
    );
  }
  await mesurer(
    "tiroir-defiler",
    TIROIR.at(-1),
    `const corps = document.querySelector(".drawer .body"); corps.scrollTop = 0; window.__fait = false;
    return await window.__mesure(async () => {
      while (corps.scrollTop + corps.clientHeight < corps.scrollHeight - 1) {
        corps.scrollTop += 600;
        await new Promise((r) => requestAnimationFrame(r));
      }
      window.__fait = true;
    }, () => window.__fait);`,
    { gel: SEUIL_GEL_MS },
  );
}

async function phototheque() {
  if (!["phototheque-lot", "phototheque-ouvrir", "phototheque-enfant"].some(voulue)) return;
  // Une classe aux prénoms inventés ; « Adèle » (eleve-0) vient en tête de la liste et figure sur une photo sur cinq.
  const prenoms = ["Adèle", "Basile", "Capucine", "Dorian", "Elsa", "Félix", "Garance", "Hugo", "Iris", "Jules", "Kenza", "Lucien", "Maëlle", "Nils", "Olympe", "Paul", "Quitterie", "Raphaël", "Salomé", "Timéo", "Ulysse", "Victoire", "Wassim", "Yanis", "Zélie"].slice(0, ELEVES);
  await run(`
    const e = window.__editor, c = window.__carnet.classes;
    e.drawer = null;
    if (!c.current) await c.createNamed("Classe de mesure", "2026-2027");
    const noms = ${JSON.stringify(prenoms)};
    c.update((k) => ({ ...k, students: noms.map((first_name, i) => ({ id: "eleve-" + i, first_name, last_name: "", level: "MS", group: "", photo: null, photo_allowed: true, symbol: null, birth_date: null })) }));
    e.showPhototheque = true;
    for (let i = 0; i < 100 && !window.__carnet.classLibrary.classPath; i++) ${attendre(100)}
  `);
  let deja = 0;
  for (const n of PHOTOTHEQUE) {
    // On n'ajoute jamais des milliers de photos d'un coup : la photothèque est remplie sans mesure, puis on mesure
    // l'ajout d'une sortie (un lot) par-dessus.
    const remplir = photos.slice(deja, n - LOT);
    const sortie = photos.slice(n - LOT, n);
    await run(`const l = window.__carnet.classLibrary; await l.add(${JSON.stringify(remplir)});`, [], 60 * 60 * 1000);
    await mesurer(
      "phototheque-lot",
      n,
      `const l = window.__carnet.classLibrary, lot = ${JSON.stringify(sortie)}; window.__fait = false;
      const r = await window.__mesure(async () => { await l.add(lot); window.__fait = true; }, () => window.__fait, 600000);
      const ids = l.photos.filter((_, i) => i % 5 === 0).map((p) => p.id);
      await l.tag(ids, { add_students: ["eleve-0"] });
      return r;`,
      { gel: SEUIL_GEL_MS },
      { prealable: true },
    );
    deja = n;
    await mesurer(
      "phototheque-ouvrir",
      n,
      `const e = window.__editor, l = window.__carnet.classLibrary;
      e.showPhototheque = false; ${attendre(500)} l.classPath = null; l.photos = [];
      return await window.__mesure(() => { e.showPhototheque = true; }, () => window.__vignettesPretes(".library-page .photos", ${n}));`,
      { ms: SEUIL_MS },
    );
    await mesurer(
      "phototheque-enfant",
      n,
      // Les photos de l'enfant sont souvent déjà chargées (le navigateur charge d'avance autour de l'écran) :
      // prêt quand la grille ne montre plus qu'elles (chaque vignette nomme ses élèves dans son infobulle), et que
      // celles qui sont visibles sont chargées. Le compte attendu n'est pas recalculé ici : la période affichée
      // en écarte certaines.
      `const grille = () => [...document.querySelectorAll(".library-page .photos .thumb")];
      const r = await window.__mesure(() => document.querySelector(".library-page .filters .child").click(), () => {
        const g = grille();
        return g.length > 0 && g.every((t) => t.title.split(", ").includes("Adèle")) && window.__vignettesPretes(".library-page .photos", g.length);
      });
      document.querySelector(".library-page .filters > button").click(); return r;`,
      { ms: SEUIL_MS },
    );
  }
}

function echapper(t) {
  return String(t).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

function ecrire() {
  const echecs = resultats.filter((r) => r.echec).length;
  const erreurs = resultats.filter((r) => r.erreur).length;
  const cas = resultats.map((r) => {
    const temps = r.ms != null ? (r.ms / 1000).toFixed(3) : "0";
    const dedans = r.erreur ? `<error message="${echapper(r.erreur)}"/>` : r.echec ? `<failure message="${echapper(r.echec)}"/>` : "";
    return `  <testcase classname="images" name="${r.nom}" time="${temps}">${dedans}</testcase>`;
  });
  writeFileSync(
    join(out, "mesures.xml"),
    `<?xml version="1.0" encoding="UTF-8"?>\n<testsuite name="mesures-images" tests="${resultats.length}" failures="${echecs}" errors="${erreurs}">\n${cas.join("\n")}\n</testsuite>\n`,
  );
  const lignes = resultats.map((r) =>
    `| ${r.scene} | ${r.taille} | ${r.erreur ? "erreur" : r.ms} | ${r.erreur ? "" : r.gelMaxMs} | ${r.memoireMo ?? "?"} | ${r.erreur ? `erreur : ${r.erreur}` : (r.echec ?? "ok")} |`,
  );
  writeFileSync(
    join(out, "mesures.md"),
    [
      "Limites : 2 cœurs, 4 Go de mémoire. Le disque dur classique n'est pas simulé. Mémoire : application et processus web, pointe depuis le lancement.",
      "",
      "| Scène | Photos | Temps (ms) | Plus long gel (ms) | Mémoire de pointe cumulée (Mo) | Résultat |",
      "|---|---|---|---|---|---|",
      ...lignes,
      "",
    ].join("\n"),
  );
}

try {
  s = await session(APP);
  await sleep(3000);
  await s.size(1600, 1000);
  await s.js(SONDE);
  await tiroir();
  await phototheque();
} finally {
  ecrire();
  if (s) await s.end().catch(() => {});
}
