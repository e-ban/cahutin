<script lang="ts">
  // Écran d'accueil : un bandeau (logo, nom, version), puis deux colonnes, « Commencer » un document et
  // « Reprendre » les récents (aperçu de leur première page), et en bas « Mes ressources ». Le même écran sert à
  // Fichier > Nouveau.
  import { getVersion } from "@tauri-apps/api/app";
  import Icon from "../Icon.svelte";
  import PageView from "../PageView.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { guides } from "../../lib/guides.svelte";
  import { pictoUrl } from "../../lib/pictos.svelte";
  import { listExamples, openExample, previewDocument } from "../../lib/api";
  import { clearRecents, forgetRecent, freshFromHome, importFiles, openDoc, openExampleDoc, openPath, trashRecent } from "../../lib/fileActions";
  import type { Document, ExampleInfo } from "../../lib/types";
  import { fr } from "../../lib/i18n/fr";
  import { modal } from "../../lib/modal.svelte";
  import { templates } from "../../lib/templates.svelte";

  const MINI = 0.24;
  /** Une miniature tient dans la case d'une page A4 portrait, même en paysage ou en A3. */
  const miniZoom = (p: { size: { width_mm: number; height_mm: number } }) => MINI * Math.min(1, 210 / p.size.width_mm, 297 / p.size.height_mm);
  const RECENT = 0.18;
  let version = $state("");
  getVersion().then((v) => (version = v)).catch(() => (version = ""));
  // Aperçus des récents, chargés une fois par document ; null : illisible ou introuvable (une icône à la place).
  let previews = $state<Record<string, Document | null>>({});
  $effect(() => {
    for (const r of editor.recents) {
      if (r.path in previews) continue;
      previews[r.path] = null;
      previewDocument(r.path).then((d) => (previews[r.path] = d)).catch(() => {});
    }
  });
  let step = $state<"choices" | "layouts" | "examples">("choices");
  /** Menu du clic droit sur un document récent. */
  let menu = $state<{ path: string; title: string; x: number; y: number } | null>(null);

  /** Action du menu sur son document, lu avant de fermer le menu : fermé, il n'est plus lisible. */
  function fromMenu(action: (r: { path: string; title: string }) => unknown) {
    const r = menu;
    menu = null;
    if (r) action(r);
  }

  /** Échap ferme le menu du clic droit, sinon revient aux choix, sinon ferme l'accueil. */
  function onEscape(e: KeyboardEvent) {
    if (e.key !== "Escape" || modal.message !== null || editor.showClassPage || editor.showCollection || editor.showPreferences) return;
    if (menu) menu = null;
    else if (step !== "choices") step = "choices";
    else close();
  }
  // Aperçus des exemples : chargés une fois, à la première visite de l'étape.
  let examples = $state<{ info: ExampleInfo; doc: Document }[]>([]);

  async function showExamples() {
    step = "examples";
    if (examples.length) return;
    try {
      const infos = await listExamples();
      examples = await Promise.all(infos.map(async (info) => ({ info, doc: await openExample(info.id) })));
    } catch {
      examples = [];
    }
  }

  async function chooseLayout(id: string) {
    if (!(await freshFromHome())) return;
    await templates.addPage(id, true);
    close();
  }

  async function blankPage() {
    if (!(await freshFromHome())) return;
    close();
  }

  /** Un modèle à soi : une page vierge en mode modèle (grille, cases à fusionner, puis « Enregistrer comme modèle »). */
  async function createTemplate() {
    if (!(await freshFromHome())) return;
    close();
    editor.templateMode = true;
    editor.drawer = "grid";
    editor.selection = [];
  }

  function close() {
    editor.showWelcome = false;
    editor.welcomeOverDoc = false;
    step = "choices";
  }

  const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;
</script>

<!-- Les choix de l'accueil : un pictogramme OpenMoji en couleur, le même sur tous les ordinateurs. -->
{#snippet pic(code: string)}<img class="pic" src={pictoUrl(code)} alt="" />{/snippet}

{#snippet layout(t: (typeof templates.infos)[number])}
  <button class="card" onclick={() => chooseLayout(t.id)}>
    <div class="mini">
      {#if templates.previews[t.id]}<PageView page={templates.previews[t.id]} doc={editor.doc} zoom={miniZoom(templates.previews[t.id])} />{/if}
    </div>
    <span>{t.name}</span>
  </button>
{/snippet}

<!-- La page « Ma classe », par-dessus, garde Échap pour elle. -->
<svelte:window onkeydown={onEscape} />

<div class="welcome">
  {#if step === "choices"}
    <header class="band">
      <img class="logo" src="/logo/cahutin.svg" alt="" />
      <div class="name">
        <span class="app">{fr.appName}</span>
        {#if version}<small>{fr.aboutVersion(version)}</small>{/if}
      </div>
      <h1>{fr.welcomeTitle}</h1>
      <button class="prefs" title={fr.preferencesHint} onclick={() => (editor.showPreferences = true)}><Icon name="settings" /> {fr.preferences}</button>
      {#if editor.welcomeOverDoc}
        <button class="back-doc" onclick={close}><Icon name="back" /> {fr.welcomeBackToDoc(editor.doc.title)}</button>
      {/if}
    </header>
    <div class="columns">
      <!-- À gauche, pour un nouveau document : partir d'un modèle d'abord, puis les autres départs. -->
      <section class="start" aria-labelledby="welcome-start">
        <h2 id="welcome-start">{fr.welcomeStart}</h2>
        <button class="choice main" onclick={() => (step = "layouts")}>
          {@render pic("1F4D0")}<span>{fr.welcomeTemplate}</span><small>{fr.welcomeTemplateSub}</small>
        </button>
        <button class="choice" onclick={blankPage}>{@render pic("1F4C4")}<span>{fr.welcomeBlank}</span><small>{fr.welcomeBlankSub}</small></button>
        <button class="choice" onclick={showExamples}>{@render pic("2728")}<span>{fr.welcomeExamples}</span><small>{fr.welcomeExamplesSub}</small></button>
        <button class="choice" onclick={() => importFiles(undefined, { fresh: true })}>{@render pic("1F4E5")}<span>{fr.welcomeImport}</span><small>{fr.welcomeImportSub}</small></button>
      </section>
      <!-- À droite, pour continuer : les documents récents, puis ouvrir un autre fichier. -->
      <section class="resume" aria-labelledby="welcome-resume">
        <div class="recents-head">
          <h2 id="welcome-resume">{fr.welcomeContinue}</h2>
          {#if editor.recents.length}<button class="plain" onclick={clearRecents}>{fr.recentsClear}</button>{/if}
        </div>
        {#if editor.recents.length === 0}
          <p class="hint">{fr.noRecents}</p>
        {:else}
          <ul class="recents">
            {#each editor.recents as r (r.path)}
              {@const doc = previews[r.path]}
              <li oncontextmenu={(e) => (e.preventDefault(), (menu = { path: r.path, title: r.title, x: e.clientX, y: e.clientY }))}>
                <button class="card" onclick={() => openPath(r.path)} title={fileName(r.path)}>
                  <div class="mini small">
                    {#if doc}<PageView page={doc.pages[0]} {doc} zoom={RECENT} />{:else}<span class="blank"><Icon name="document" size={40} /></span>{/if}
                  </div>
                  <span class="title">{r.title}</span>
                  <small>{new Date(r.openedAt).toLocaleDateString("fr-FR")}</small>
                </button>
                <button class="forget" title={fr.recentForget} aria-label={`${fr.recentForget} : ${r.title}`} onclick={() => forgetRecent(r.path)}><Icon name="close" size={14} /></button>
              </li>
            {/each}
          </ul>
          <p class="hint small-hint">{fr.recentsHint}</p>
        {/if}
        <button class="choice" onclick={() => openDoc()}>{@render pic("1F4C2")}<span>{fr.welcomeResume}</span><small>{fr.welcomeResumeSub}</small></button>
      </section>
    </div>
    <!-- Ce qui sert à tous les documents : la classe, la collection d'images, les guides. -->
    <section class="own" aria-labelledby="welcome-own">
      <h2 id="welcome-own">{fr.welcomeOwn}</h2>
      <div class="own-row">
        <button class="choice" onclick={() => (editor.showClassPage = true)}>{@render pic("1F9D2")}<span>{fr.welcomeClass}</span><small>{fr.welcomeClassSub}</small></button>
        <button class="choice" onclick={() => (editor.showCollection = true)}>{@render pic("1F5BC")}<span>{fr.welcomeCollection}</span><small>{fr.welcomeCollectionSub}</small></button>
        <button class="choice" onclick={() => guides.show()}>{@render pic("1F4D8")}<span>{fr.welcomeGuides}</span><small>{fr.welcomeGuidesSub}</small></button>
      </div>
    </section>
    {#if menu}
      {@const m = menu}
      <!-- Menu du clic droit : un clic ailleurs ou Échap le ferme. -->
      <div class="menu-backdrop" role="presentation" onclick={() => (menu = null)} oncontextmenu={(e) => (e.preventDefault(), (menu = null))}></div>
      <div class="menu" role="menu" style:left="{m.x}px" style:top="{m.y}px">
        <button role="menuitem" onclick={() => fromMenu((r) => openPath(r.path))}>{fr.recentOpen}</button>
        <button role="menuitem" onclick={() => fromMenu((r) => forgetRecent(r.path))}>{fr.recentForget}</button>
        <button role="menuitem" class="danger" onclick={() => fromMenu((r) => trashRecent(r.path, r.title))}><Icon name="trash" /> {fr.recentTrashMenu}</button>
      </div>
    {/if}
  {:else if step === "examples"}
    <div class="head">
      <button class="back" onclick={() => (step = "choices")}><Icon name="back" /> {fr.classPageBack}</button>
      <h1>{fr.chooseExample}</h1>
    </div>
    <div class="grid">
      {#each examples as ex (ex.info.id)}
        <button class="card" onclick={() => openExampleDoc(ex.info.id)}>
          <div class="mini"><PageView page={ex.doc.pages[0]} doc={ex.doc} zoom={miniZoom(ex.doc.pages[0])} /></div>
          <span>{ex.info.name}</span>
        </button>
      {/each}
    </div>
  {:else}
    <div class="head">
      <button class="back" onclick={() => (step = "choices")}><Icon name="back" /> {fr.classPageBack}</button>
      <h1>{fr.chooseTemplate}</h1>
    </div>
    <div class="grid">
      <button class="card new" onclick={createTemplate}>
        <div class="mini"><span class="plus"><Icon name="add" size={64} /></span></div>
        <span>{fr.createTemplate}</span>
        <small>{fr.createTemplateSub}</small>
      </button>
      {#each templates.infos.filter((t) => !t.for_class) as t (t.id)}{@render layout(t)}{/each}
    </div>
    <h2 title={fr.templatesForClassHint}>{fr.templatesForClass}</h2>
    <div class="grid">
      {#each templates.infos.filter((t) => t.for_class) as t (t.id)}{@render layout(t)}{/each}
    </div>
  {/if}
</div>

<style>
  .welcome {
    position: fixed;
    inset: 0;
    background: var(--bg);
    padding: 32px clamp(24px, 4vw, 64px) 48px;
    overflow: auto;
    z-index: 5000;
  }
  .band {
    display: flex;
    align-items: center;
    gap: 18px;
    margin-bottom: 28px;
  }
  .logo {
    width: 96px;
    height: 96px;
  }
  .name {
    display: flex;
    flex-direction: column;
    margin-right: auto;
  }
  .app {
    font: 700 40px var(--title-font);
    color: var(--accent-strong);
    line-height: 1.1;
  }
  .name small {
    margin-top: 4px;
    color: var(--muted);
    font-size: 13px;
  }
  h1 {
    font: 600 26px var(--ui-font);
    margin: 0 0 24px;
  }
  .prefs {
    background: var(--surface);
  }
  .back-doc {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }
  .band h1 {
    margin: 0;
    font-weight: 500;
    font-size: 22px;
    color: var(--muted);
  }
  h2 {
    margin: 0 0 12px;
    font-size: 15px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
  }
  /* Deux colonnes : commencer (étroite) et reprendre (large) ; une seule quand la fenêtre est étroite. */
  .columns {
    display: grid;
    grid-template-columns: minmax(280px, 380px) 1fr;
    gap: 36px;
    align-items: start;
  }
  @media (max-width: 900px) {
    .columns {
      grid-template-columns: 1fr;
    }
  }
  .start,
  .resume {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
  }
  /* Un choix : le pictogramme à gauche, le nom et sa précision à droite. */
  .choice {
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-rows: auto auto;
    column-gap: 14px;
    row-gap: 1px;
    align-items: center;
    padding: 10px 16px;
    background: var(--panel);
    border-radius: 12px;
    text-align: left;
    font-size: 15px;
    font-weight: 600;
  }
  .choice .pic {
    grid-row: span 2;
    width: 44px;
    height: 44px;
  }
  .choice span,
  .choice small {
    justify-self: start;
  }
  .choice small {
    color: var(--muted);
    font-weight: 400;
    font-size: 12.5px;
  }
  /* Partir d'un modèle : le départ conseillé, plus grand. */
  .choice.main {
    padding: 18px 16px;
    border: 2px solid var(--accent);
    background: color-mix(in srgb, var(--accent) 7%, var(--panel));
    font-size: 17px;
  }
  .choice.main .pic {
    width: 64px;
    height: 64px;
  }
  .resume > .choice {
    align-self: start;
    margin-top: 6px;
  }
  .own {
    margin-top: 36px;
    padding-top: 22px;
    border-top: 1px solid var(--line);
  }
  .own-row {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 14px;
  }
  @media (max-width: 900px) {
    .own-row {
      grid-template-columns: 1fr;
    }
  }
  .recents {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 18px;
  }
  .recents .card {
    width: 100%;
    gap: 6px;
  }
  .recents li {
    position: relative;
  }
  /* Toujours visible pour qu'on la trouve, rouge au survol. */
  .forget {
    position: absolute;
    top: 6px;
    right: 6px;
    width: 24px;
    height: 24px;
    padding: 0;
    border-radius: 50%;
    border: 1px solid var(--line);
    background: color-mix(in srgb, var(--surface) 95%, transparent);
    color: var(--muted);
    display: grid;
    place-items: center;
  }
  .forget:hover,
  .forget:focus-visible {
    background: var(--danger);
    border-color: var(--danger);
    color: var(--surface);
  }
  .recents-head {
    display: flex;
    align-items: baseline;
    gap: 16px;
  }
  .plain {
    border-color: transparent;
    background: transparent;
    color: var(--muted);
    text-decoration: underline;
  }
  .small-hint {
    margin-top: 10px;
    font-size: 13px;
  }
  .menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
  }
  .menu {
    position: fixed;
    z-index: 51;
    display: flex;
    flex-direction: column;
    min-width: 200px;
    padding: 4px;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 8px;
    box-shadow: 0 6px 20px var(--shadow);
  }
  .menu button {
    text-align: left;
    border: 0;
    background: transparent;
    padding: 7px 10px;
  }
  .menu button:hover {
    background: var(--accent-soft);
  }
  .menu .danger {
    color: var(--danger);
  }
  .menu .danger:hover {
    background: var(--danger-soft);
  }
  .recents .title {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .recents small,
  .hint {
    color: var(--muted);
  }
  .recents small {
    font-weight: 400;
  }
  .mini.small {
    width: 144px;
    height: 204px;
    align-items: center;
    background: white;
    box-shadow: 0 1px 4px var(--shadow);
  }
  .card.new {
    border: 2px dashed var(--accent);
  }
  .card.new .mini {
    align-items: center;
    background: var(--accent-soft);
  }
  .card.new small {
    color: var(--muted);
    font-weight: 400;
  }
  .plus {
    display: flex;
    color: var(--accent);
  }
  .blank {
    display: flex;
    opacity: 0.4;
  }
  .head {
    display: flex;
    align-items: baseline;
    gap: 16px;
  }
  .back {
    border-color: transparent;
    background: transparent;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
    gap: 18px;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    align-items: center;
    padding: 8px;
    background: var(--panel);
    font-weight: 600;
  }
  .mini {
    width: 190px;
    height: 270px;
    overflow: hidden;
    pointer-events: none;
    display: flex;
    justify-content: center;
    align-items: center;
  }
</style>
