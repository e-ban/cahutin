// Les images des guides (assets/guides) : chaque scène part d'un exemple de Cahutin ou d'une page neuve, règle
// l'interface, entoure en orange ce que l'étape désigne, puis capture. 1000 px de large, en WebP ; les animations
// sont faites de plusieurs captures.
// Usage : node guides.mjs <dossier assets/guides> [guide/image…]
// Variables : IMAGES (dossier des images de test, voir images.sh), HOME_CAP (dossier Documents jetable).
import { execFileSync } from "node:child_process";
import { mkdirSync } from "node:fs";
import { session, sleep } from "./wd.mjs";

const out = process.argv[2];
const only = process.argv.slice(3);
const APP = new URL("../../target/debug/cahutin", import.meta.url).pathname;
const IMAGES = process.env.IMAGES;
const W = 1500;
const H = 816;
let s;
let shots = 0;

const run = (body, args = []) =>
  s.jsAsync(`const done = arguments[arguments.length - 1]; (async () => { ${body} })().then((v) => done(v ?? null), (e) => done("ERREUR " + e))`, args);

async function check(v) {
  if (typeof v === "string" && v.startsWith("ERREUR")) throw new Error(v);
  return v;
}

/** Ouvre un exemple (ou une page neuve si `example` est nul), interface remise à zéro. */
async function open(example = null) {
  await check(
    await run(
      `const e = window.__editor, c = window.__carnet;
      e.showPreferences = false; e.inspectorOpen = false; e.cropping = null; e.drawer = null; e.selection = [];
      if (arguments[0]) await c.openExampleDoc(arguments[0]);
      e.showWelcome = false; e.zoom = "fit";`,
      [example],
    ),
  );
  await sleep(1200);
}

/** Entoure en orange : un sélecteur CSS, ou { sel, text } pour l'élément de ce type qui contient ce texte. */
async function mark(...specs) {
  await s.js(
    `if (!document.getElementById("guide-mark-style")) {
      const st = document.createElement("style");
      st.id = "guide-mark-style";
      st.textContent = ".guide-mark { outline: 3px solid #e8731a !important; outline-offset: 2px !important; border-radius: 6px; }";
      document.head.append(st);
    }
    for (const spec of arguments[0]) {
      const all = [...document.querySelectorAll(typeof spec === "string" ? spec : spec.sel)];
      const els = typeof spec === "string" ? all : all.filter((e) => e.textContent.includes(spec.text)).slice(-1);
      els.forEach((e) => e.classList.add("guide-mark"));
    }`,
    [specs],
  );
  await sleep(300);
}

const clickText = async (sel, text) => {
  await s.js("const e = [...document.querySelectorAll(arguments[0])].filter((b) => b.textContent.includes(arguments[1])).pop(); e?.click()", [sel, text]);
  await sleep(700);
};
const click = async (sel) => {
  await s.js("document.querySelector(arguments[0])?.click()", [sel]);
  await sleep(700);
};
const select = async (ids) => {
  await run(`window.__editor.selection = arguments[0];`, [ids]);
  await sleep(700);
};
const ids = (filter) => s.js(`return window.__editor.page.elements.filter(${filter}).map((x) => x.id)`);

/** Remplit la première case photo vide avec une image de test. */
async function fillPhoto(file = "photos/jardin-1.jpg") {
  return check(
    await run(
      `const e = window.__editor, c = window.__carnet;
      await c.importPaths([arguments[0]], { quiet: true });
      const meta = e.photos.find((p) => p.path === arguments[0])?.meta ?? e.photos.at(-1)?.meta;
      const box = e.page.elements.find((x) => x.kind.type === "Photo" && !x.kind.asset);
      if (box && meta) e.placePhoto(box.id, meta);
      return box?.id;`,
      [`${IMAGES}/${file}`],
    ),
  );
}

/** Capture : la fenêtre entière, ou un morceau { x, y, w, h }, ramené à 1000 px de large. */
async function shot(name, crop = null) {
  // Un fichier par capture : les images d'une animation sont gardées jusqu'à son assemblage.
  const png = `${out}/.tmp-${name.replace("/", "-")}-${++shots}.png`;
  await s.shot(png);
  const dst = `${out}/${name}.webp`;
  mkdirSync(dst.slice(0, dst.lastIndexOf("/")), { recursive: true });
  const cropArgs = crop ? ["-crop", `${crop.w}x${crop.h}+${crop.x}+${crop.y}`, "+repage"] : [];
  execFileSync("magick", [png, ...cropArgs, "-resize", "1000x", "-quality", "82", dst]);
  // Pour une animation : l'image recadrée comme l'image fixe.
  if (!crop) return png;
  const cropped = png.replace(/\.png$/, "-recadree.png");
  execFileSync("magick", [png, ...cropArgs, cropped]);
  return cropped;
}

/** Animation : des captures successives, chacune affichée `delay` centièmes de seconde. */
async function animate(name, frames, delays) {
  const dst = `${out}/${name}.webp`;
  const args = frames.flatMap((f, i) => ["-delay", String(delays[i] ?? delays.at(-1)), f]);
  execFileSync("magick", [...args, "-loop", "0", "-resize", "1000x", "-quality", "80", dst]);
}

async function templateMode() {
  await run(`const e = window.__editor; e.templateMode = true; e.drawer = "grid"; e.selection = [];`);
  await sleep(800);
}

const scenes = {
  // Créer un modèle
  async "modele/mode"() {
    await open();
    await templateMode();
    await mark('[data-guide="template-mode"]', ".banner");
    await shot("modele/mode");
  },
  async "modele/grille"() {
    await open();
    await templateMode();
    const frames = [await shot("modele/grille")];
    await run(`const e = window.__editor, c = window.__carnet; e.updatePage(() => e.page); const p = await c.api.gridSet(e.page, 2, 3); e.commit({ ...e.doc, pages: e.doc.pages.map((x, i) => (i === e.pageIndex ? p : x)) });`);
    await sleep(900);
    frames.push(await shot("modele/grille"));
    const cells = await s.js("return window.__editor.page.grid.cells.map((c) => c.element_id)");
    await select([cells[0], cells[1]]);
    await mark(`main.stage [data-id="${cells[0]}"]`, `main.stage [data-id="${cells[1]}"]`);
    frames.push(await shot("modele/grille"));
    await run(`const e = window.__editor, c = window.__carnet; const cells = e.page.grid.cells; const p = await c.api.gridMerge(e.page, cells[0].element_id, cells[1].element_id); e.commit({ ...e.doc, pages: e.doc.pages.map((x, i) => (i === e.pageIndex ? p : x)) }); e.selection = [];`);
    await sleep(900);
    await s.js("document.querySelectorAll('.guide-mark').forEach((e) => e.classList.remove('guide-mark'))");
    frames.push(await shot("modele/grille"));
    await animate("modele/grille", frames, [120, 150, 150, 250]);
  },
  async "modele/cases"() {
    await open();
    await templateMode();
    await run(`const e = window.__editor, c = window.__carnet; const p = await c.api.gridSet(e.page, 2, 3); e.commit({ ...e.doc, pages: e.doc.pages.map((x, i) => (i === e.pageIndex ? p : x)) });`);
    await sleep(800);
    const cell = await s.js("return window.__editor.page.grid.cells[0].element_id");
    await select([cell]);
    await mark({ sel: '[aria-label="Barre de la case"] button', text: "Photo" }, { sel: '[aria-label="Barre de la case"] button', text: "Texte" });
    await shot("modele/cases");
  },
  async "modele/role"() {
    await open("ferme");
    await templateMode();
    const [title] = await ids("(x) => x.kind.type === 'Text' && x.frame.y_mm < 40");
    await select([title]);
    await clickText(".ctx .role button", "Fixe");
    await mark(".ctx .role");
    await shot("modele/role");
  },
  async "modele/garde"() {
    await open("ferme");
    const photo = await fillPhoto();
    await templateMode();
    await select([photo]);
    await mark(".ctx .role");
    await shot("modele/garde");
  },
  async "modele/enregistrer"() {
    await open();
    await templateMode();
    await run(`const e = window.__editor, c = window.__carnet; const p = await c.api.gridSet(e.page, 2, 3); e.commit({ ...e.doc, pages: e.doc.pages.map((x, i) => (i === e.pageIndex ? p : x)) });`);
    await sleep(800);
    await mark('[data-guide="save-template"]');
    await shot("modele/enregistrer");
  },

  // Remplir avec ma classe
  async "classe/ouvrir"() {
    await run(`window.__editor.showWelcome = true;`);
    await sleep(800);
    await mark({ sel: "button.choice", text: "Ma classe" });
    await shot("classe/ouvrir");
  },
  async "classe/autorisations"() {
    await run(`window.__editor.showWelcome = false; window.__editor.showClassPage = true;`);
    await sleep(1000);
    await mark('[data-guide="class-consent"]', { sel: ".class-page button", text: "Symboles pour tous" });
    await shot("classe/autorisations");
  },
  async "classe/bandeau"() {
    await open("porte-manteaux");
    await mark({ sel: ".banner button", text: "Remplir avec" });
    await shot("classe/bandeau");
  },
  async "classe/remplir"() {
    await open("porte-manteaux");
    await clickText(".banner button", "Remplir avec");
    await sleep(800);
    await mark(".box .primary", ".box .who");
    await shot("classe/remplir");
  },

  // Photothèque
  async "phototheque/ouvrir"() {
    await open("ferme");
    await run(`window.__editor.drawer = "class";`);
    await sleep(700);
    await mark('[data-guide="phototheque"]');
    await shot("phototheque/ouvrir");
  },
  async "phototheque/ajouter"() {
    await libraryWithPhotos();
    await mark('[data-guide="library-add"]', { sel: ".library-page button", text: "Dossier" });
    await shot("phototheque/ajouter");
  },
  async "phototheque/qui"() {
    await libraryWithPhotos();
    await chooseLibraryPhotos(3);
    await mark(".library-page aside.panel");
    await shot("phototheque/qui");
  },
  async "phototheque/retrouver"() {
    await libraryWithPhotos({ tagged: true });
    await clickText(".library-page .filters button, .library-page nav button, .library-page aside button", "Inès");
    await mark(".library-page .filters, .library-page nav.filters, .library-page .side");
    await shot("phototheque/retrouver");
  },
  async "phototheque/document"() {
    await libraryWithPhotos({ tagged: true });
    await chooseLibraryPhotos(2);
    await mark({ sel: ".library-page button", text: "dans le document" });
    await shot("phototheque/document");
  },

  // Flouter des visages : sur une illustration de classe (FLOU_IMAGE), jamais une vraie photo d'enfants.
  async "flou/choisir"() {
    await blurPhoto();
    await mark('[data-guide="blur"]');
    await shot("flou/choisir");
  },
  async "flou/accord"() {
    await blurPhoto();
    await click('[data-guide="blur"]');
    await sleep(800);
    await mark(".box .consent .buttons");
    await shot("flou/accord", await rect(".box"));
  },
  async "flou/tracer"() {
    await blurDialog();
    const crop = await rect(".box");
    const frames = [await shot("flou/tracer", crop)];
    for (const i of [0, 2, 4]) {
      await s.js("document.querySelectorAll('.box .face')[arguments[0]]?.click()", [i]);
      await sleep(500);
      frames.push(await shot("flou/tracer", crop));
    }
    await animate("flou/tracer", frames, [150, 90, 90, 250]);
  },
  async "flou/resultat"() {
    await blurDialog();
    for (const i of [0, 2, 4]) await s.js("document.querySelectorAll('.box .face')[arguments[0]]?.click()", [i]);
    await sleep(500);
    await mark(".box .buttons .primary");
    await shot("flou/resultat", await rect(".box"));
  },

  // Étiquettes à découper
  async "etiquettes/bloc"() {
    await oneLabel();
    await mark(".ctx");
    await shot("etiquettes/bloc");
  },
  async "etiquettes/repeter"() {
    await oneLabel();
    await click('.ctx button[aria-haspopup="menu"]');
    await click('[data-guide="repeat"]');
    await mark(".pop.repeat");
    await shot("etiquettes/repeter");
  },
  async "etiquettes/reperes"() {
    await oneLabel();
    await click('.ctx button[aria-haspopup="menu"]');
    await click('[data-guide="repeat"]');
    await clickText(".pop.repeat button", "Remplir la page");
    await sleep(800);
    await click('.ctx button[aria-haspopup="menu"]');
    await click('[data-guide="cut"]');
    await mark(".pop.list");
    await shot("etiquettes/reperes");
  },
  async "etiquettes/imprimer"() {
    await open("porte-manteaux");
    await clickText(".top .menu button.trigger", "Imprimer");
    await mark(".top .menu .list");
    await shot("etiquettes/imprimer");
  },

  // Reprendre une ancienne fiche
  async "import/importer"() {
    await open("ferme");
    await clickText(".top .menu button.trigger", "Fichier");
    await mark({ sel: ".top .menu .list .item", text: "Importer" });
    await shot("import/importer");
  },

  async "import/resultat"() {
    await importedPage();
    const [text] = await ids("(x) => x.kind.type === 'Text' && x.frame.y_mm > 40");
    await select([text]);
    await mark(".ctx");
    await shot("import/resultat");
  },
  async "import/verifier"() {
    await importedPage();
    await mark(".notice");
    await shot("import/verifier");
  },

  // Frise
  async "frise/motifs"() {
    await open();
    await run(`window.__editor.drawer = "decor";`);
    await sleep(900);
    await s.js("const d = document.querySelector('details#decor-friezes, details[data-id=decor-friezes]') ?? [...document.querySelectorAll('details.section')].find((x) => x.textContent.includes('Frises')); if (d) d.open = true;");
    await sleep(600);
    await mark('[data-guide="frieze-pick"]');
    await shot("frise/motifs");
  },
  async "frise/autour"() {
    await open("ferme");
    const photo = await fillPhoto();
    await select([photo]);
    await click('.ctx button[aria-haspopup="menu"]');
    await mark('[data-guide="frieze-around"]');
    await shot("frise/autour");
  },
  async "frise/taille"() {
    await open("ferme");
    await fillPhoto();
    const frieze = await ids("(x) => x.kind.type === 'Image' && x.group === window.__editor.page.elements.find((y) => y.kind.type === 'Image' && y.group).group");
    await select(frieze);
    await click('.ctx button[aria-haspopup="menu"]');
    await click('[data-guide="frieze-size"]');
    await mark(".pop.frieze");
    await shot("frise/taille");
  },

  // Ma collection
  async "collection/ajouter"() {
    await open();
    await run(`window.__editor.openImages("mine");`);
    await sleep(900);
    await mark('[data-guide="collection-add"]');
    await shot("collection/ajouter");
  },
  async "collection/ranger"() {
    await collectionWithImages();
    await run(`window.__editor.showCollection = true;`);
    await sleep(1200);
    await s.js("[...document.querySelectorAll('.collection-page .pic')].slice(0, 2).forEach((b) => b.click())");
    await sleep(500);
    await s.js("const i = document.querySelector('.collection-page input[list], .collection-page .bar input'); if (i) { i.value = 'Automne'; i.dispatchEvent(new Event('input', { bubbles: true })); }");
    await sleep(400);
    await mark({ sel: ".collection-page div", text: "Ranger dans le thème" });
    await shot("collection/ranger");
  },
  async "collection/poser"() {
    await collectionWithImages();
    await run(`window.__editor.openImages("mine");`);
    await sleep(1000);
    await mark({ sel: ".drawer div", text: "Automne" });
    await shot("collection/poser");
  },

  // L'assistant IA : avec Ollama sur l'ordinateur (OLLAMA_URL), une vraie proposition.
  async "assistant/activer"() {
    await assistantOn();
    await open("sens");
    await run(`window.__editor.prefsCategory = "assistant"; window.__editor.showPreferences = true;`);
    await sleep(1000);
    await mark(".pane .explain");
    await shot("assistant/activer");
  },
  async "assistant/demander"() {
    await assistantOn();
    await open("sens");
    await run(`window.__carnet.assistant.open = true;`);
    await sleep(800);
    await s.js("const t = document.querySelector('aside.right textarea'); t.value = 'Mets le titre en rouge et plus gros.'; t.dispatchEvent(new Event('input', { bubbles: true }));");
    await sleep(400);
    await mark("aside.right form, aside.right .ask, aside.right textarea");
    await shot("assistant/demander");
  },
  async "assistant/accepter"() {
    await assistantProposal();
    await mark({ sel: ".banner button", text: "Accepter" }, { sel: ".banner button", text: "Refuser" });
    await shot("assistant/accepter");
  },
  async "assistant/signaler"() {
    await assistantProposal();
    await mark({ sel: "aside.right button", text: "Signaler" });
    await shot("assistant/signaler");
  },
};

// Cette scène ferme la classe pour montrer la page de création : elle passe après toutes les autres.
scenes["classe/coller"] = async () => {
  await run(`const c = window.__carnet.classes; await c.close(); window.__editor.showWelcome = false; window.__editor.showClassPage = true;`);
  await sleep(1000);
  await s.js(
    "const t = document.querySelector('.class-page textarea'); t.value = 'Léo\\nInès\\nTom\\nLina\\nNoah\\nJade\\nMalo\\nZoé'; t.dispatchEvent(new Event('input', { bubbles: true }));",
  );
  await sleep(400);
  await mark(".class-page textarea", { sel: ".class-page button", text: "Créer ma classe" });
  await shot("classe/coller");
  await run(`await window.__carnet.classes.restore();`);
};

/** La photothèque de la classe, avec les photos de test ; `tagged` : des enfants et une activité déjà notés. */
async function libraryWithPhotos({ tagged = false } = {}) {
  await open("ferme");
  await check(
    await run(
      `const c = window.__carnet, lib = c.classLibrary;
      await lib.open(c.classes.current.path);
      if (!lib.photos.length) await lib.follow(arguments[0]);
      if (arguments[1]) {
        const ids = (kind) => lib.photos.filter((p) => p.path.split(/[\\/]/).pop().startsWith(kind)).map((p) => p.id);
        await lib.tag(ids("peinture-"), { add_students: ["eleve-1", "eleve-3"], add_activities: ["Peinture"] });
        await lib.tag(ids("jardin-"), { add_students: ["eleve-1", "eleve-5"], add_activities: ["Jardin"] });
      }
      window.__editor.showPhototheque = true;`,
      [`${IMAGES}/photos`, tagged],
    ),
  );
  await sleep(1500);
}

async function chooseLibraryPhotos(n) {
  await s.js(`[...document.querySelectorAll('.library-page .photo, .library-page .thumb')].slice(0, ${n}).forEach((b, i) => b.dispatchEvent(new MouseEvent("click", { bubbles: true, ctrlKey: i > 0 })))`);
  await sleep(800);
}

/** Une fiche LibreOffice reprise : l'exemple « Les saisons » exporté en ODG, puis importé dans un document neuf. */
async function importedPage() {
  const path = `${process.env.HOME_CAP}/Les saisons.odg`;
  await open("saisons");
  await check(await run(`const c = window.__carnet, e = window.__editor; await c.api.exportOdg(arguments[0], e.doc); await c.openExampleDoc("ferme");`, [path]));
  await sleep(800);
  await check(await run(`await window.__carnet.importFiles([arguments[0]], { fresh: true });`, [path]));
  await sleep(2500);
}

/** Le rectangle d'un élément à l'écran, pour ne capturer que lui (une fenêtre de dialogue). */
const rect = (sel) => s.js("const r = document.querySelector(arguments[0]).getBoundingClientRect(); return { x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height) };", [sel]);

/** L'illustration de classe dans la première case photo de « Sortie à la ferme », choisie. */
async function blurPhoto() {
  await open("ferme");
  const photo = await check(
    await run(
      `const e = window.__editor, c = window.__carnet;
      await c.importPaths([arguments[0]], { quiet: true });
      const meta = e.photos.at(-1)?.meta;
      const box = e.page.elements.find((x) => x.kind.type === "Photo" && !x.kind.asset);
      e.placePhoto(box.id, meta);
      return box.id;`,
      [process.env.FLOU_IMAGE],
    ),
  );
  await select([photo]);
}

/** La fenêtre « Flouter des visages », le repérage accepté et fini. */
async function blurDialog() {
  await blurPhoto();
  await click('[data-guide="blur"]');
  await sleep(800);
  await clickText(".box .consent .buttons button", "Oui");
  await click(".box .consent .buttons .primary");
  for (let i = 0; i < 40 && (await s.js("return !!document.querySelector('.box .busy')")); i++) await sleep(500);
  await sleep(800);
}

/** Une page avec une seule étiquette (la première de l'exemple porte-manteaux), choisie. */
async function oneLabel() {
  await open("porte-manteaux");
  await run(`const e = window.__editor;
    const first = e.page.elements.find((x) => x.group);
    e.updatePage((p) => ({ ...p, grid: null, elements: p.elements.filter((x) => x.group === first.group) }));
    e.selection = e.page.elements.map((x) => x.id);`);
  await sleep(900);
}

async function collectionWithImages() {
  await check(
    await run(
      `const m = window.__carnet.myImages;
      await m.load();
      if (m.images.length < 5) await m.add(arguments[0]);
      await m.theme(["pomme.png", "citrouille.png", "feuille.png"], "Automne");
      await m.theme(["poisson.png", "chat.png"], "Animaux");`,
      [["pomme", "citrouille", "feuille", "poisson", "chat"].map((n) => `${IMAGES}/collection/${n}.png`)],
    ),
  );
  await open();
}

async function assistantOn() {
  await check(
    await run(
      `const c = window.__carnet;
      await c.api.assistantConfigure(true, arguments[0], "qwen3:4b", "");
      await c.assistant.load();`,
      [process.env.OLLAMA_URL ?? "http://localhost:11434/v1"],
    ),
  );
}

/** Une vraie proposition de l'assistant sur « Mes cinq sens » : elle prend un moment. */
async function assistantProposal() {
  await assistantOn();
  await open("sens");
  // La demande n'est pas attendue ici : un script WebDriver n'a que 30 secondes ; on surveille la fin.
  await run(`const a = window.__carnet.assistant; a.open = true; void a.send("Mets le titre en rouge et plus gros, puis ajoute un cadre de page fleuri.");`);
  await sleep(2000);
  for (let i = 0; i < 150 && (await s.js("return window.__carnet.assistant.busy")); i++) await sleep(2000);
  await sleep(800);
}

// Une première ouverture : pas de visite guidée, d'astuces ni de nouveautés, et la classe d'exemple.
s = await session(APP);
await sleep(3000);
await check(
  await run(`
  for (const [k, v] of [["tourDone", true], ["hintsEnabled", false], ["classIntroSeen", true], ["updateConsent", false], ["whatsNewSeen", window.__carnet.whatsNew.version]])
    localStorage.setItem("carnet." + k, JSON.stringify(v));
  const c = window.__carnet.classes;
  if (!c.current) await c.createNamed("Moyenne section", "2026-2027");
  const names = [["Léo", "MS"], ["Inès", "MS"], ["Tom", "GS"], ["Lina", "MS"], ["Noah", "GS"], ["Jade", "MS"], ["Malo", "GS"], ["Zoé", "MS"]];
  c.update((k) => ({ ...k, students: names.map(([first_name, level], i) => ({ id: "eleve-" + i, first_name, last_name: "", level, group: i % 3 === 0 ? "Les loups" : "", photo: null, photo_allowed: i !== 4, symbol: null, birth_date: null })) }));
  await c.flush();
`),
);
await s.end();

// Une application neuve par scène.
for (const [name, scene] of Object.entries(scenes)) {
  if (only.length && !only.includes(name)) continue;
  s = await session(APP);
  try {
    await sleep(3500);
    await s.size(W, H);
    await sleep(500);
    await scene();
    console.log("image :", name);
  } catch (e) {
    console.log("ÉCHEC :", name, String(e).slice(0, 300));
  } finally {
    await s.end().catch(() => {});
  }
}
