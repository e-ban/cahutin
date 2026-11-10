// Les captures de la release : chaque scène part d'un exemple de Cahutin, règle l'interface, puis capture.
// Usage : node scenes.mjs <dossier de sortie> [scène…]
import { session, sleep } from "./wd.mjs";

const out = process.argv[2];
const only = process.argv.slice(3);
// Une photo de test générée (paysage dessiné par ImageMagick), jamais une vraie photo.
const PHOTO = process.env.PHOTO;
const APP = new URL("../../target/debug/cahutin", import.meta.url).pathname;
let s;

const run = (body, args = []) => s.jsAsync(`const done = arguments[arguments.length - 1]; (async () => { ${body} })().then((v) => done(v ?? null), (e) => done("ERREUR " + e))`, args);

/** Remet l'interface à zéro et ouvre un exemple. */
async function open(example, { theme = "papier" } = {}) {
  await run(`
    const e = window.__editor, c = window.__carnet;
    c.settings.set("theme", ${JSON.stringify(theme)});
    e.showPreferences = false; e.inspectorOpen = false; e.cropping = null; e.drawer = null; e.selection = [];
    if (c.guides.close) c.guides.close();
    await c.openExampleDoc(${JSON.stringify(example)});
    e.showWelcome = false;
    e.zoom = "fit";
  `);
  await sleep(1500);
}

/** Une photo de test dans la première case photo vide. */
async function fillPhoto() {
  return run(`
    const e = window.__editor, c = window.__carnet;
    if (!e.photos.length) await c.importPaths([${JSON.stringify(PHOTO)}], { quiet: true });
    const box = e.page.elements.find((x) => x.kind.type === "Photo" && !x.kind.asset);
    if (box && e.photos.length) e.placePhoto(box.id, e.photos[0].meta);
    return box?.id;
  `);
}

/** Clique un élément de la page par son identifiant. */
async function clickElement(id, group = false) {
  await run(
    `const e = window.__editor, id = arguments[0], group = arguments[1];
    const target = e.page.elements.find((x) => x.id === id);
    e.selection = group && target?.group ? e.page.elements.filter((x) => x.group === target.group).map((x) => x.id) : [id];`,
    [id, group],
  );
  await sleep(800);
}

async function clickText(selector, text) {
  const ok = await s.js(
    "const el = [...document.querySelectorAll(arguments[0])].find((b) => b.textContent.trim().includes(arguments[1])); if (el) el.click(); return !!el",
    [selector, text],
  );
  await sleep(600);
  return ok;
}

const scenes = {
  async welcome() {
    await s.size(1440, 1032);
    await run(`window.__carnet.settings.set("theme", "papier"); window.__editor.showWelcome = true;`);
    await sleep(800);
  },
  async readme() {
    await s.size(1400, 762);
    await open("anniversaire");
  },
  async "site-anniversaire"() {
    await s.size(1440, 1032);
    await open("anniversaire");
    await run(`window.__editor.drawer = "templates";`);
    await sleep(1200);
    await s.js("const h = [...document.querySelectorAll('h2, h3, h4')].find((x) => x.textContent.includes('Pour la classe')); h?.scrollIntoView({ block: 'start' })");
    await sleep(600);
  },
  async "site-decors"() {
    await s.size(1440, 1032);
    await open("emotions");
    await run(`window.__editor.openImages("pictos");`);
    await sleep(1200);
    await clickText("button", "Émotions");
  },
  async frise() {
    await s.size(1896, 1032);
    await open("ferme");
    await fillPhoto();
    const frieze = await s.js("return window.__editor.page.elements.find((x) => x.kind.type === 'Image' && x.group)?.id");
    await clickElement(frieze, true);
    // Le réglage de la frise est dans « Plus » (réglage par défaut).
    await s.js("document.querySelector('.ctx button[aria-haspopup=\"menu\"]')?.click()");
    await sleep(500);
    await s.js("document.querySelector('[data-guide=\"frieze-size\"]')?.click()");
    await sleep(800);
  },
  async reglages() {
    await s.size(1896, 1032);
    await open("ferme");
    const id = await fillPhoto();
    await clickElement(id);
    await run(`window.__editor.inspectorOpen = true;`);
    await sleep(800);
  },
  async rogner() {
    await s.size(1896, 1032);
    await open("ferme");
    const id = await fillPhoto();
    await clickElement(id);
    await run(`window.__editor.cropping = ${JSON.stringify(id)};`);
    await sleep(800);
  },
  async preferences() {
    await s.size(1896, 1032);
    await open("ferme");
    await fillPhoto();
    await run(`window.__editor.prefsCategory = "display"; window.__editor.showPreferences = true;`);
    await sleep(800);
  },
  async guides() {
    await s.size(1896, 1032);
    await open("ferme");
    await run(`window.__carnet.guides.show("modele");`);
    await sleep(1200);
  },
  async nuit() {
    await s.size(1896, 1032);
    await open("ferme", { theme: "nuit" });
    const id = await fillPhoto();
    await clickElement(id);
    await sleep(500);
  },
  // Le Store : 1896 × 1032.
  async "store-modeles"() {
    await scenes["site-anniversaire"]();
    await s.size(1896, 1032);
    await sleep(800);
  },
  async "store-images"() {
    await scenes["site-decors"]();
    await s.size(1896, 1032);
    await sleep(800);
  },
  async "store-exemples"() {
    await s.size(1896, 1032);
    await run(`window.__editor.showWelcome = true;`);
    await sleep(600);
    await clickText("button.choice", "Découvrir un exemple");
    await sleep(2500);
  },
  async "store-accueil"() {
    await s.size(1896, 1032);
    // Quatre exemples enregistrés dans le dossier personnel des captures, rouverts : ils deviennent récents.
    await run(`
      const c = window.__carnet, e = window.__editor, invoke = window.__TAURI_INTERNALS__.invoke;
      const home = ${JSON.stringify(process.env.HOME_CAP)};
      for (const [id, name] of [["ferme", "Sortie à la ferme"], ["meteo", "La météo de la semaine"], ["emotions", "Le loto des émotions"], ["anniversaire", "Joyeux anniversaire"]]) {
        await c.openExampleDoc(id);
        const path = home + "/" + name + ".carnet";
        await invoke("save_document", { path, doc: { ...e.doc, title: name } });
        await c.openPath(path);
      }
      e.showWelcome = true;
    `);
    await sleep(2500);
  },
  async "store-etiquettes"() {
    await s.size(1896, 1032);
    await open("porte-manteaux");
    await run(`
      const c = window.__carnet.classes;
      if (!c.current) await c.createNamed("Moyenne section", "2026-2027");
      const names = ["Léo", "Inès", "Tom", "Lina", "Noah", "Jade", "Malo", "Zoé"];
      c.update((k) => ({ ...k, students: names.map((first_name, i) => ({ id: "eleve-" + i, first_name, last_name: "", level: "MS", group: "", photo: null, photo_allowed: false, symbol: null, birth_date: null })) }));
    `);
    await sleep(1200);
    await s.js("[...document.querySelectorAll('button')].find((b) => b.textContent.includes('Remplir avec'))?.click()");
    await sleep(1500);
    await s.js("document.querySelector('.box .primary')?.click()");
    await sleep(2500);
    await s.js("document.querySelector('.notice button')?.click()");
    await run(`window.__editor.selection = [];`);
    await sleep(600);
  },
  // Les nouveautés de la 0.16 : 1200 × 740, comme celles des versions précédentes.
  async "nouv-look"() {
    // L'accueil avec des documents récents, comme le verra quelqu'un qui met à jour.
    await scenes["store-accueil"]();
    await s.size(1200, 740);
    await sleep(1000);
  },
  async "nouv-themes"() {
    await s.size(1200, 740);
    await open("ferme", { theme: "nuit" });
    const id = await fillPhoto();
    await clickElement(id);
    await sleep(500);
  },
  async "nouv-prefs"() {
    await s.size(1200, 740);
    await open("ferme");
    await run(`window.__editor.prefsCategory = "accessibility"; window.__editor.showPreferences = true;`);
    await sleep(800);
    await s.js("const i = document.querySelector('nav input[type=search]'); i.value = 'contraste'; i.dispatchEvent(new Event('input', { bubbles: true }));");
    await sleep(600);
  },
  async "nouv-eleves"() {
    await s.size(1200, 740);
    // Une classe d'exemple : des prénoms inventés, sans photo.
    await run(`
      const c = window.__carnet.classes;
      if (!c.current) await c.createNamed("Moyenne section", "2026-2027");
      const names = [["Léo", "MS"], ["Inès", "MS"], ["Tom", "GS"], ["Lina", "MS"], ["Noah", "GS"], ["Jade", "MS"], ["Malo", "GS"], ["Zoé", "MS"]];
      c.update((k) => ({ ...k, students: names.map(([first_name, level], i) => ({ id: "eleve-" + i, first_name, last_name: "", level, group: i % 3 === 0 ? "Les loups" : "", photo: null, photo_allowed: true, symbol: null, birth_date: null })) }));
      window.__editor.showWelcome = false;
      window.__editor.showClassPage = true;
    `);
    await sleep(1200);
    await s.js("[...document.querySelectorAll('td.pick-col input')].filter((_, i) => [1, 3, 5].includes(i)).forEach((b) => b.click());");
    await sleep(600);
  },
  async "barre-du-bas"() {
    await s.size(1440, 900);
    await open("ferme");
    await sleep(400);
  },
};

// Une première ouverture : pas de visite guidée, d'astuces ni de nouveautés sur les captures (le dossier personnel
// est neuf), et une classe d'exemple aux prénoms inventés, pour que le bandeau « Remplir avec … » soit le même partout.
s = await session(APP);
await sleep(3000);
await run(`
  for (const [k, v] of [["tourDone", true], ["hintsEnabled", false], ["classIntroSeen", true], ["updateConsent", false], ["whatsNewSeen", window.__carnet.whatsNew.version]])
    localStorage.setItem("carnet." + k, JSON.stringify(v));
  const c = window.__carnet.classes;
  if (!c.current) await c.createNamed("Moyenne section", "2026-2027");
  const names = [["Léo", "MS"], ["Inès", "MS"], ["Tom", "GS"], ["Lina", "MS"], ["Noah", "GS"], ["Jade", "MS"], ["Malo", "GS"], ["Zoé", "MS"]];
  c.update((k) => ({ ...k, students: names.map(([first_name, level], i) => ({ id: "eleve-" + i, first_name, last_name: "", level, group: i % 3 === 0 ? "Les loups" : "", photo: null, photo_allowed: true, symbol: null, birth_date: null })) }));
  await c.flush();
`);
await s.end();

// Une application neuve par scène : une page modifiée ferait demander « Enregistrer ? » à la suivante.
for (const [name, scene] of Object.entries(scenes)) {
  if (only.length && !only.includes(name)) continue;
  s = await session(APP);
  try {
    await sleep(4000);
    await scene();
    await s.shot(`${out}/${name}.png`);
    console.log("capture :", name);
  } finally {
    await run(`window.__carnet.settings.set("theme", "papier");`).catch(() => {});
    await s.end().catch(() => {});
  }
}
