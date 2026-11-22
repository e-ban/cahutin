// Les captures de la release : chaque scène part d'un exemple de Cahutin, règle l'interface, puis capture.
// Usage : node scenes.mjs <dossier de sortie> [scène…]
import { execFileSync } from "node:child_process";
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

/** Un cahier de vie de treize pages prises dans les exemples, rangé en chapitres (la couverture n'en a pas). */
async function cahier() {
  await run(`
    const c = window.__carnet, e = window.__editor;
    const plan = [["prenom"], ["regles", "Septembre", "#2e86de"], ["emotions"], ["sens"], ["ferme", "Sortie à la ferme", "#27ae60"], ["saisons"],
      ["anniversaire", "Octobre", "#e67e22"], ["meteo"], ["jours"], ["semaine", "Novembre", "#8e44ad"], ["anniversaire"], ["emotions", "Décembre", "#c0392b"], ["meteo"]];
    const pages = [], assets = {};
    for (const [id, title, color] of plan) {
      await c.openExampleDoc(id);
      pages.push({ ...e.doc.pages[0], chapter: title ? { title, color } : null });
      Object.assign(assets, e.doc.assets);
    }
    e.reset({ ...e.doc, title: "Cahier de vie", pages, assets }, null);
    e.showWelcome = false; e.zoom = "fit"; e.selection = [];
  `);
  await sleep(1200);
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
    // Les animaux du pré : la frise, un groupe d'au moins trois images.
    const frieze = await s.js("const els = window.__editor.page.elements.filter((x) => x.kind.type === 'Image' && x.group); return els.find((x) => els.filter((y) => y.group === x.group).length >= 3)?.id");
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
        await invoke("save_document", { path, doc: { ...e.doc, title: name }, mode: "Replace", keepStart: false });
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
  // Une vraie proposition de l'assistant, avec Ollama sur l'ordinateur (qwen3:4b).
  async "store-fleches"() {
    await s.size(1896, 1032);
    await open("sens");
    // Une flèche accrochée de la carte mentale, choisie, et le choix de sa pointe.
    const id = await s.js("return window.__editor.page.elements.find((x) => x.kind.type === 'Shape' && x.kind.attach)?.id");
    await clickElement(id);
    await clickText(".ctx button", "Fin");
  },
  async "store-assistant"() {
    await s.size(1896, 1032);
    await open("sens");
    await run(`const c = window.__carnet; await c.api.assistantConfigure(true, arguments[0], "qwen3:4b", ""); await c.assistant.load();`, [process.env.OLLAMA_URL ?? "http://localhost:11434/v1"]);
    // La demande n'est pas attendue : un script WebDriver n'a que 30 secondes ; on surveille la fin.
    await run(`const a = window.__carnet.assistant; a.open = true; void a.send("Mets le titre en rouge et plus gros, puis ajoute un cadre de page fleuri.");`);
    await sleep(2000);
    for (let i = 0; i < 150 && (await s.js("return window.__carnet.assistant.busy")); i++) await sleep(2000);
    await sleep(800);
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
  // Les nouveautés de la 0.17 et de la 0.18 : fenêtre de 1440 × 888, ramenée à 1200 × 740 (la barre du haut
  // tient en entier). « nouv-pages » : 1500 × 925, le coin bas gauche (929 × 573) agrandi à 1200 × 740.
  async "nouv-puces"() {
    await s.size(1440, 888);
    await open("ferme");
    const id = await run(`
      const e = window.__editor;
      const p = (text, list) => ({ align: null, runs: [{ text, bold: false, italic: false, underline: false, color: null, font: null, size_pt: null }], list: list ? "Bullet" : null });
      const t = e.page.elements.find((x) => x.kind.type === "Text" && Math.round(x.frame.y_mm) === 210);
      e.updateElement(t.id, (x) => ({ ...x, frame: { ...x.frame, y_mm: 200, h_mm: 72 }, kind: { ...x.kind, prompt: null,
        style: { ...x.kind.style, size_pt: 16 },
        content: { paragraphs: [p("Ce que nous avons vu :"), p("les chèvres et leurs petits", true), p("le tracteur rouge du fermier", true), p("les œufs des poules", true)] },
        frame: { fill: "#ffffff", stroke: null, corner_radius_mm: 3, pattern: "Seyes", write_on_lines: true } } }));
      e.zoom = 1;
      return t.id;
    `);
    await clickElement(id);
    await s.js("document.querySelector(`[data-id=\"${arguments[0]}\"]`)?.scrollIntoView({ block: 'center' })", [id]);
    await sleep(400);
    await clickText(".ctx button", "Cadre");
  },
  async "nouv-trop-long"() {
    await s.size(1440, 888);
    await open("ferme");
    const id = await run(`
      const e = window.__editor;
      const t = e.page.elements.find((x) => x.kind.type === "Text" && x.kind.content.paragraphs.some((p) => p.runs.some((r) => r.text.includes("chèvres"))));
      const text = "J'ai donné à manger aux chèvres ! Elles avaient très faim et la plus petite a mangé dans ma main.";
      e.updateElement(t.id, (x) => ({ ...x, kind: { ...x.kind, content: { paragraphs: [{ align: null, runs: [{ text, bold: false, italic: false, underline: false, color: null, font: null, size_pt: null }], list: null }] } } }));
      e.zoom = 1;
      return t.id;
    `);
    await clickElement(id);
    await s.js("document.querySelector(`[data-id=\"${arguments[0]}\"]`)?.scrollIntoView({ block: 'center' })", [id]);
    await sleep(600);
  },
  async "nouv-pages"() {
    await s.size(1500, 925);
    // Un cahier de six pages, prises dans les exemples ; la page 5 glissée entre la 2 et la 3, pas encore lâchée.
    await run(`
      const c = window.__carnet, e = window.__editor;
      const pages = [], assets = {};
      for (const id of ["ferme", "saisons", "meteo", "sens", "jours", "prenom"]) { await c.openExampleDoc(id); pages.push(...e.doc.pages); Object.assign(assets, e.doc.assets); }
      e.reset({ ...e.doc, title: "Cahier de vie", pages, assets }, null);
      // À 53 %, le bord du recadrage tombe entre deux lignes d'écriture de la page.
      e.showWelcome = false; e.zoom = 0.53; e.selection = [];
      e.goToPage(5);
    `);
    await sleep(1500);
    await run(`
      const thumbs = [...document.querySelectorAll(".thumb")];
      const b = thumbs[4].getBoundingClientRect(), t = thumbs[1].getBoundingClientRect();
      const ev = (type, x, y, target) => target.dispatchEvent(new PointerEvent(type, { bubbles: true, clientX: x, clientY: y, pointerId: 1, pointerType: "mouse", button: 0, isPrimary: true }));
      ev("pointerdown", b.left + b.width / 2, b.top + b.height / 2, thumbs[4]);
      for (let k = 1; k <= 10; k++) {
        ev("pointermove", b.left + b.width / 2 + ((t.left - b.left) * k) / 10, b.top + b.height / 2 - 4, window);
        await new Promise((r) => setTimeout(r, 30));
      }
    `);
    await sleep(400);
  },
  async "nouv-enregistrement"() {
    await s.size(1440, 888);
    // Un exemple enregistré dans le dossier personnel des captures : il a un fichier, il peut s'enregistrer seul.
    await run(`
      const c = window.__carnet, e = window.__editor, invoke = window.__TAURI_INTERNALS__.invoke;
      await c.openExampleDoc("ferme");
      const path = ${JSON.stringify(process.env.HOME_CAP)} + "/Sortie à la ferme.carnet";
      await invoke("save_document", { path, doc: { ...e.doc, title: "Sortie à la ferme" }, mode: "Replace", keepStart: false });
      await c.openPath(path);
      e.showWelcome = false; e.zoom = "fit";
    `);
    await sleep(800);
    // La case « Automatique », à côté du bouton « Enregistrer ».
    await s.js("const b = document.querySelector('header.top label.auto input'); if (b && !b.checked) b.click();");
    await sleep(600);
  },
  async "nouv-bord"() {
    await s.size(1440, 888);
    await open("ferme");
    // La chèvre à 3 mm du bord, et l'avis tel qu'il s'affiche quand on imprime (sans ouvrir la fenêtre d'impression).
    await run(`
      const e = window.__editor;
      const goat = e.page.elements.find((x) => x.kind.type === "Image" && x.frame.w_mm > 30);
      e.updateElement(goat.id, (x) => ({ ...x, frame: { ...x.frame, x_mm: 163, y_mm: 240 } }));
      const { printWarning } = await import("/src/lib/printSafety.ts");
      const w = printWarning(e.doc, { linux: false, pdf: false, tiles: false });
      e.notify(w.message, { label: "Montrer", run: () => {} });
    `);
    await sleep(600);
  },
  async "nouv-graphisme"() {
    await s.size(1440, 888);
    await run(`
      const c = window.__carnet, e = window.__editor;
      c.settings.set("theme", "papier");
      for (const [k, v] of [["graphismeDots", "progressif"], ["graphismeGuides", "Corridor"], ["graphismeArrow", true], ["graphismeRows", 4]]) localStorage.setItem("carnet." + k, JSON.stringify(v));
      const { newDocument } = await import("/src/lib/factory.ts");
      const { inspectorSections } = await import("/src/lib/inspectorSections.svelte.ts");
      e.reset(newDocument("Graphisme"), null);
      e.showWelcome = false; e.zoom = "fit"; e.selection = [];
      inspectorSections.set("shapes-graphisme", true);
      e.drawer = "shapes";
    `);
    await sleep(800);
    await s.js("[...document.querySelectorAll('button')].find((b) => b.title?.startsWith('Ajouter des lignes de vagues'))?.click()");
    await sleep(500);
    // Le tiroir refermé : la page seule, de près.
    await run(`const e = window.__editor; e.selection = []; e.drawer = null; e.zoom = 1;`);
    await sleep(400);
  },
  async "nouv-regler"() {
    await s.size(1440, 888);
    await run(`
      const c = window.__carnet, e = window.__editor;
      c.settings.set("theme", "papier");
      for (const [k, v] of [["graphismeDots", "progressif"], ["graphismeGuides", "Corridor"], ["graphismeArrow", true], ["graphismeRows", 4]]) localStorage.setItem("carnet." + k, JSON.stringify(v));
      const { newDocument } = await import("/src/lib/factory.ts");
      const { inspectorSections } = await import("/src/lib/inspectorSections.svelte.ts");
      e.reset(newDocument("Graphisme"), null);
      e.showWelcome = false; e.zoom = "fit"; e.selection = [];
      inspectorSections.set("shapes-graphisme", true);
      e.drawer = "shapes";
    `);
    await sleep(800);
    await s.js("[...document.querySelectorAll('button')].find((b) => b.title?.startsWith('Ajouter des lignes de ponts'))?.click()");
    await sleep(500);
    // La troisième ligne choisie, comme d'un clic : ses motifs vides apparaissent en gris clair.
    await run(`
      const e = window.__editor;
      const groups = [...new Set(e.page.elements.map((x) => x.group))];
      e.drawer = null; e.zoom = 1;
      e.selection = e.page.elements.filter((x) => x.group === groups[2]).map((x) => x.id);
    `);
    await sleep(600);
    await s.js("document.querySelector('.ctx button[aria-haspopup=\"menu\"]')?.click()");
    await sleep(400);
    await clickText(".ctx [role=menuitem]", "Régler les lignes");
  },
  // Les nouveautés de la 0.19 : fenêtre de 1440 × 888, ramenée à 1200 × 740.
  async "nouv-fleches"() {
    await s.size(1440, 888);
    await open("sens");
    // Une flèche accrochée de la carte mentale, choisie : ses deux bouts ronds, et le choix de sa pointe.
    const id = await run(`
      const e = window.__editor;
      const a = e.page.elements.find((x) => x.kind.type === "Shape" && x.kind.attach);
      e.zoom = 1;
      return a?.id;
    `);
    await clickElement(id);
    await s.js("document.querySelector(`[data-id=\"${arguments[0]}\"]`)?.scrollIntoView({ block: 'center' })", [id]);
    await sleep(600);
    // Le menu « Fin » ouvert : les pointes au choix.
    await clickText(".ctx button", "Fin");
  },
  async "nouv-chemins"() {
    await s.size(1440, 888);
    await open("meteo");
    // La flèche courbe « Aujourd'hui », choisie : ses points se tirent un par un.
    const id = await run(`
      const e = window.__editor;
      const a = e.page.elements.find((x) => x.kind.type === "Shape" && x.kind.points && x.kind.curve);
      e.zoom = 1;
      return a?.id;
    `);
    await clickElement(id);
    await s.js("document.querySelector(`[data-id=\"${arguments[0]}\"]`)?.scrollIntoView({ block: 'center' })", [id]);
    await sleep(600);
  },
  async "nouv-bulles"() {
    await s.size(1440, 888);
    await open("ferme");
    // La bulle qui vise la chèvre, choisie : le losange jaune sur sa pointe.
    const id = await run(`
      const e = window.__editor;
      const b = e.page.elements.find((x) => x.kind.type === "Shape" && x.kind.shape === "Bubble");
      e.zoom = 1;
      return b?.id;
    `);
    await clickElement(id);
    await s.js("document.querySelector(`[data-id=\"${arguments[0]}\"]`)?.scrollIntoView({ block: 'center' })", [id]);
    await sleep(600);
  },
  async "nouv-exemples"() {
    await s.size(1440, 888);
    await run(`window.__carnet.settings.set("theme", "papier"); window.__editor.showWelcome = true;`);
    await sleep(800);
    await clickText(".welcome button", "exemple");
    await sleep(2500);
  },
  async "nouv-reglages"() {
    await s.size(1440, 888);
    await open("ferme");
    // Rien de choisi, Réglages ouvert : le panneau règle la page.
    await run(`const e = window.__editor; e.selection = []; e.inspectorOpen = true;`);
    await sleep(800);
  },
  async "nouv-chapitres"() {
    await s.size(1440, 888);
    await cahier();
    // La page de la sortie, sa photo posée : son chapitre est déplié dans la bande, les autres repliés sur leur onglet.
    await run(`window.__editor.goToPage(5);`);
    await sleep(800);
    await fillPhoto();
    await run(`window.__editor.selection = [];`);
    await sleep(1200);
  },
  async "nouv-toutes-pages"() {
    await s.size(1440, 888);
    await cahier();
    await run(`window.__editor.showAllPages = true;`);
    await sleep(2000);
  },
  async "nouv-imprimer-chapitre"() {
    await s.size(1440, 888);
    await cahier();
    await run(`window.__editor.goToPage(8);`);
    await sleep(800);
    await clickText(".top button", "Imprimer");
    await sleep(800);
  },
  async "barre-du-bas"() {
    await s.size(1440, 900);
    await open("ferme");
    await sleep(400);
  },
};

// Les pages des exemples, pour le site : la page seule, rognée autour de son bord (A4 à 640 px de large,
// paysage à 902). La scène rend le cadre de la page, en pixels de la fenêtre.
for (const id of ["ferme", "anniversaire", "meteo", "prenom", "saisons", "semaine", "emotions", "jours", "regles", "sens", "porte-manteaux"]) {
  scenes[`page-${id}`] = async () => {
    await s.size(1500, 1500);
    await open(id);
    await run(`const e = window.__editor; e.drawer = null; e.zoom = 297 / Math.max(e.page.size.width_mm, e.page.size.height_mm);`);
    await sleep(1200);
    return s.js("const r = document.querySelector('.page').getBoundingClientRect(); return { crop: [r.left, r.top, r.width, r.height].map(Math.round), width: r.width > r.height ? 902 : 640 };");
  };
}

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
    const r = await scene();
    await s.shot(`${out}/${name}.png`);
    if (r?.crop) {
      const [x, y, w, h] = r.crop;
      execFileSync("magick", [`${out}/${name}.png`, "-crop", `${w}x${h}+${x}+${y}`, "+repage", "-resize", `${r.width}x`, `${out}/${name}.png`]);
    }
    console.log("capture :", name);
  } finally {
    // Le thème et l'enregistrement automatique (décoché au départ) reviennent à leur réglage d'origine.
    await run(`window.__carnet.settings.set("theme", "papier"); window.__carnet.settings.set("autosave", false);`).catch(() => {});
    await s.end().catch(() => {});
  }
}
