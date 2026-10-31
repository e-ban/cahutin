<script lang="ts">
  // Écran d'accueil : trois grands choix et les documents récents. Le même écran sert à Fichier > Nouveau.
  import PageView from "../PageView.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { listExamples, openExample } from "../../lib/api";
  import { importFiles, openDoc, openExampleDoc, openPath } from "../../lib/fileActions";
  import type { Document, ExampleInfo } from "../../lib/types";
  import { fr } from "../../lib/i18n/fr";
  import { modal } from "../../lib/modal.svelte";
  import { templates } from "../../lib/templates.svelte";

  const MINI = 0.24;
  let step = $state<"choices" | "layouts" | "examples">("choices");
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
    await templates.addPage(id, true);
    close();
  }

  function close() {
    editor.showWelcome = false;
    step = "choices";
  }

  const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;
</script>

<!-- La page « Ma classe », par-dessus, garde Échap pour elle. -->
<svelte:window onkeydown={(e) => e.key === "Escape" && modal.message === null && !editor.showClassPage && (step !== "choices" ? (step = "choices") : close())} />

<div class="welcome">
  {#if step === "choices"}
    <h1>{fr.welcomeTitle}</h1>
    <div class="choices">
      <button class="choice main" onclick={() => (step = "layouts")}>
        <b>📐</b><span>{fr.welcomeTemplate}</span><small>{fr.welcomeTemplateSub}</small>
      </button>
      <button class="choice" onclick={close}><b>📄</b><span>{fr.welcomeBlank}</span><small>{fr.welcomeBlankSub}</small></button>
      <button class="choice" onclick={() => openDoc()}><b>📂</b><span>{fr.welcomeResume}</span><small>{fr.welcomeResumeSub}</small></button>
      <button class="choice" onclick={() => importFiles(undefined, { fresh: true })}><b>📥</b><span>{fr.welcomeImport}</span><small>{fr.welcomeImportSub}</small></button>
      <button class="choice" onclick={showExamples}><b>✨</b><span>{fr.welcomeExamples}</span><small>{fr.welcomeExamplesSub}</small></button>
      <button class="choice" onclick={() => (editor.showClassPage = true)}><b>🧒</b><span>{fr.welcomeClass}</span><small>{fr.welcomeClassSub}</small></button>
    </div>
    <h2>{fr.recentDocs}</h2>
    {#if editor.recents.length === 0}
      <p class="hint">{fr.noRecents}</p>
    {:else}
      <ul class="recents">
        {#each editor.recents as r (r.path)}
          <li>
            <button onclick={() => openPath(r.path)}>
              <span class="title">{r.title}</span>
              <small>{fileName(r.path)} · {new Date(r.openedAt).toLocaleDateString("fr-FR")}</small>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  {:else if step === "examples"}
    <div class="head">
      <button class="back" onclick={() => (step = "choices")}>{fr.back}</button>
      <h1>{fr.chooseExample}</h1>
    </div>
    <div class="grid">
      {#each examples as ex (ex.info.id)}
        <button class="card" onclick={() => openExampleDoc(ex.info.id)}>
          <div class="mini"><PageView page={ex.doc.pages[0]} doc={ex.doc} zoom={MINI} /></div>
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
      {#each templates.infos as t (t.id)}
        <button class="card" onclick={() => chooseLayout(t.id)}>
          <div class="mini">
            {#if templates.previews[t.id]}<PageView page={templates.previews[t.id]} doc={editor.doc} zoom={MINI} />{/if}
          </div>
          <span>{t.name}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .welcome {
    position: fixed;
    inset: 0;
    background: var(--bg);
    padding: 48px max(24px, calc(50vw - 480px));
    overflow: auto;
    z-index: 5000;
  }
  h1 {
    font: 600 26px var(--ui-font);
    margin: 0 0 24px;
  }
  h2 {
    margin: 36px 0 10px;
  }
  .choices {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 16px;
  }
  .choice {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 24px 12px;
    background: var(--panel);
    border-radius: 12px;
    font-size: 16px;
    font-weight: 600;
  }
  .choice b {
    font-size: 40px;
    font-weight: 400;
  }
  .choice small {
    color: var(--muted);
    font-weight: 400;
    font-size: 13px;
  }
  .choice.main {
    border: 2px solid var(--accent);
    background: #f2f8fe;
  }
  .recents {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .recents button {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
    text-align: left;
    padding: 10px 14px;
    background: white;
  }
  .recents small,
  .hint {
    color: var(--muted);
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
  }
</style>
