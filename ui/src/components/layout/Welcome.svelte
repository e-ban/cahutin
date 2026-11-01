<script lang="ts">
  // Écran d'accueil : un bandeau (logo, nom, version), les choix sur toute la largeur, puis les documents
  // récents avec l'aperçu de leur première page. Le même écran sert à Fichier > Nouveau.
  import { getVersion } from "@tauri-apps/api/app";
  import PageView from "../PageView.svelte";
  import { editor } from "../../lib/docStore.svelte";
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

  /** Échap ferme le menu du clic droit, sinon revient aux choix, sinon ferme l'accueil. */
  function onEscape(e: KeyboardEvent) {
    if (e.key !== "Escape" || modal.message !== null || editor.showClassPage || editor.showCollection) return;
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
      {#if editor.welcomeOverDoc}
        <button class="back-doc" onclick={close}>{fr.welcomeBackToDoc(editor.doc.title)}</button>
      {/if}
    </header>
    <div class="choices">
      <button class="choice main" onclick={() => (step = "layouts")}>
        <b>📐</b><span>{fr.welcomeTemplate}</span><small>{fr.welcomeTemplateSub}</small>
      </button>
      <button class="choice" onclick={blankPage}><b>📄</b><span>{fr.welcomeBlank}</span><small>{fr.welcomeBlankSub}</small></button>
      <button class="choice" onclick={() => openDoc()}><b>📂</b><span>{fr.welcomeResume}</span><small>{fr.welcomeResumeSub}</small></button>
      <button class="choice" onclick={() => importFiles(undefined, { fresh: true })}><b>📥</b><span>{fr.welcomeImport}</span><small>{fr.welcomeImportSub}</small></button>
      <button class="choice" onclick={showExamples}><b>✨</b><span>{fr.welcomeExamples}</span><small>{fr.welcomeExamplesSub}</small></button>
    </div>
    <!-- Ce qui sert à tous les documents, à part : la collection d'images et la classe. -->
    <div class="choices own">
      <button class="choice" onclick={() => (editor.showCollection = true)}><b>🗂</b><span>{fr.welcomeCollection}</span><small>{fr.welcomeCollectionSub}</small></button>
      <button class="choice" onclick={() => (editor.showClassPage = true)}><b>🧒</b><span>{fr.welcomeClass}</span><small>{fr.welcomeClassSub}</small></button>
    </div>
    <div class="recents-head">
      <h2>{fr.recentDocs}</h2>
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
                {#if doc}<PageView page={doc.pages[0]} {doc} zoom={RECENT} />{:else}<span class="blank">📄</span>{/if}
              </div>
              <span class="title">{r.title}</span>
              <small>{new Date(r.openedAt).toLocaleDateString("fr-FR")}</small>
            </button>
            <button class="forget" title={fr.recentForget} aria-label={`${fr.recentForget} : ${r.title}`} onclick={() => forgetRecent(r.path)}>×</button>
          </li>
        {/each}
      </ul>
      <p class="hint small-hint">{fr.recentsHint}</p>
    {/if}
    {#if menu}
      {@const m = menu}
      <!-- Menu du clic droit : un clic ailleurs ou Échap le ferme. -->
      <div class="menu-backdrop" role="presentation" onclick={() => (menu = null)} oncontextmenu={(e) => (e.preventDefault(), (menu = null))}></div>
      <div class="menu" role="menu" style:left="{m.x}px" style:top="{m.y}px">
        <button role="menuitem" onclick={() => ((menu = null), openPath(m.path))}>{fr.recentOpen}</button>
        <button role="menuitem" onclick={() => ((menu = null), forgetRecent(m.path))}>{fr.recentForget}</button>
        <button role="menuitem" class="danger" onclick={() => ((menu = null), trashRecent(m.path, m.title))}>🗑 {fr.recentTrashMenu}</button>
      </div>
    {/if}
  {:else if step === "examples"}
    <div class="head">
      <button class="back" onclick={() => (step = "choices")}>{fr.back}</button>
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
      <button class="back" onclick={() => (step = "choices")}>{fr.back}</button>
      <h1>{fr.chooseTemplate}</h1>
    </div>
    <div class="grid">
      <button class="card new" onclick={createTemplate}>
        <div class="mini"><span class="plus">＋</span></div>
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
  .back-doc {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
    font-weight: 600;
  }
  .band h1 {
    margin: 0;
    font-weight: 500;
    font-size: 22px;
    color: var(--muted);
  }
  h2 {
    margin: 36px 0 14px;
  }
  .choices {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 14px;
  }
  /* Cinq choix pour un document : une rangée si la fenêtre est large, sinon deux rangées de trois places, le
     premier choix en prenant deux. */
  @media (max-width: 1250px) {
    .choices {
      grid-template-columns: repeat(3, 1fr);
    }
    .choice.main {
      grid-column: span 2;
    }
  }
  /* La collection et la classe : une rangée à part, plus basse, l'image à gauche du texte. */
  .choices.own {
    grid-template-columns: repeat(2, 1fr);
    margin-top: 22px;
  }
  .own .choice {
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-rows: auto auto;
    column-gap: 14px;
    row-gap: 2px;
    align-items: center;
    text-align: left;
    padding: 14px 20px;
  }
  .own .choice b {
    grid-row: span 2;
  }
  .own .choice span,
  .own .choice small {
    justify-self: start;
    text-align: left;
  }
  .choice {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 20px 10px;
    background: var(--panel);
    border-radius: 12px;
    font-size: 15px;
    font-weight: 600;
  }
  .choice b {
    font-size: 36px;
    font-weight: 400;
  }
  .choice small {
    color: var(--muted);
    font-weight: 400;
    font-size: 12.5px;
    text-align: center;
  }
  .choice.main {
    border: 2px solid var(--accent);
    background: #f2f8fe;
  }
  .recents {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
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
    background: rgba(255, 255, 255, 0.95);
    color: var(--muted);
    font-size: 16px;
    line-height: 22px;
  }
  .forget:hover,
  .forget:focus-visible {
    background: #d32f2f;
    border-color: #d32f2f;
    color: white;
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
    background: white;
    border: 1px solid var(--line);
    border-radius: 8px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.15);
  }
  .menu button {
    text-align: left;
    border: 0;
    background: transparent;
    padding: 7px 10px;
  }
  .menu button:hover {
    background: #e8f1fb;
  }
  .menu .danger {
    color: #c0392b;
  }
  .menu .danger:hover {
    background: #fdecea;
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
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.12);
  }
  .card.new {
    border: 2px dashed var(--accent);
  }
  .card.new .mini {
    align-items: center;
    background: #f2f8fe;
  }
  .card.new small {
    color: var(--muted);
    font-weight: 400;
  }
  .plus {
    font-size: 64px;
    color: var(--accent);
  }
  .blank {
    font-size: 40px;
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
