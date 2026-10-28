<script lang="ts">
  // Écran d'accueil : trois grands choix et les documents récents. Le même écran sert à Fichier > Nouveau.
  import PageView from "../PageView.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { openDoc, openPath } from "../../lib/fileActions";
  import { fr } from "../../lib/i18n/fr";
  import { templates } from "../../lib/templates.svelte";

  const MINI = 0.24;
  let step = $state<"choices" | "layouts">("choices");

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

<svelte:window onkeydown={(e) => e.key === "Escape" && (step === "layouts" ? (step = "choices") : close())} />

<div class="welcome">
  {#if step === "choices"}
    <h1>{fr.welcomeTitle}</h1>
    <div class="choices">
      <button class="choice main" onclick={() => (step = "layouts")}>
        <b>📔</b><span>{fr.welcomeLifeBook}</span><small>{fr.welcomeLifeBookSub}</small>
      </button>
      <button class="choice" onclick={close}><b>📄</b><span>{fr.welcomeBlank}</span><small>{fr.welcomeBlankSub}</small></button>
      <button class="choice" onclick={() => openDoc()}><b>📂</b><span>{fr.welcomeResume}</span><small>{fr.welcomeResumeSub}</small></button>
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
  {:else}
    <div class="head">
      <button class="back" onclick={() => (step = "choices")}>{fr.back}</button>
      <h1>{fr.chooseLayout}</h1>
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
    grid-template-columns: repeat(3, 1fr);
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
