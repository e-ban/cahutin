<script lang="ts">
  import GuideLink from "../GuideLink.svelte";
  // Page de la collection : ranger ses images dans des thèmes (« Automne », « Animaux »), les renommer, les
  // retirer et remettre celles retirées, en ajouter (fichiers ou dossier entier).
  import { open } from "@tauri-apps/plugin-dialog";
  import { ask } from "../../lib/ask";
  import { listImportableIn, myImageUrl, removedImageUrl } from "../../lib/api";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { displayName, filterByTheme, themeCounts, type ThemeFilter } from "../../lib/myImages";
  import { myImages } from "../../lib/myImages.svelte";

  type View = ThemeFilter | { kind: "removed" };

  let view = $state<View>({ kind: "all" });
  let selected = $state<string[]>([]);
  let themeName = $state("");
  let error = $state<string | null>(null);

  myImages.load();
  myImages.loadRemoved();

  const themes = $derived(themeCounts(myImages.images));
  const untagged = $derived(myImages.images.filter((i) => !i.themes?.length).length);
  const shown = $derived(view.kind === "removed" ? myImages.removed : filterByTheme(myImages.images, view));
  const isView = (v: View) => JSON.stringify(v) === JSON.stringify(view);

  function show(v: View) {
    view = v;
    selected = [];
  }

  function close() {
    editor.showCollection = false;
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === "Escape" && !(e.target instanceof HTMLInputElement)) close();
  }

  const toggle = (name: string) => (selected = selected.includes(name) ? selected.filter((n) => n !== name) : [...selected, name]);

  async function run(action: () => Promise<unknown>) {
    error = null;
    try {
      await action();
    } catch (e) {
      error = String(e);
    }
  }

  async function addFiles() {
    const picked = await open({ multiple: true, filters: [{ name: fr.myImages, extensions: ["png", "jpg", "jpeg", "webp", "svg", "PNG", "JPG", "JPEG", "WEBP", "SVG"] }] });
    const paths = picked === null ? [] : Array.isArray(picked) ? picked : [picked];
    await run(async () => {
      const refused = await myImages.add(paths);
      if (refused) editor.notice = fr.myImagesRefused(refused);
    });
  }

  async function addFolder() {
    const dir = await open({ directory: true });
    if (typeof dir !== "string") return;
    await run(async () => {
      const refused = await myImages.add(await listImportableIn(dir));
      if (refused) editor.notice = fr.myImagesRefused(refused);
    });
  }

  async function file() {
    const theme = themeName.trim();
    if (!theme || !selected.length) return;
    await run(() => myImages.theme(selected, theme));
    themeName = "";
    selected = [];
  }

  async function takeOut(names: string[], theme: string) {
    await run(() => myImages.theme(names, null, theme));
    selected = [];
  }

  async function removeChosen() {
    if (!(await ask(fr.collectionRemoveConfirm(selected.length), fr.myImagesRemove))) return;
    await run(async () => {
      for (const n of selected) await myImages.remove(n);
      await myImages.loadRemoved();
    });
    selected = [];
  }

  async function rename(name: string, input: HTMLInputElement) {
    const wanted = input.value.trim();
    if (!wanted || wanted === displayName(name)) {
      input.value = displayName(name);
      return;
    }
    await run(() => myImages.rename(name, wanted));
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="collection-page">
  <div class="top">
    <button class="back" onclick={close}>← {fr.classPageBack}</button>
    <h1>🗂 {fr.collectionTitle}</h1>
    <button onclick={addFiles}>＋ {fr.myImagesAdd}</button>
    <button onclick={addFolder}>{fr.addFolderButton}</button>
    <GuideLink id="collection" />
  </div>
  <p class="intro">{fr.collectionIntro}</p>
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <div class="layout">
    <aside class="filters">
      <button class:on={isView({ kind: "all" })} onclick={() => show({ kind: "all" })}>{fr.allPictures} <span class="n">{myImages.images.length}</span></button>
      <h3>{fr.collectionThemes}</h3>
      {#each themes as t (t.theme)}
        <button class:on={isView({ kind: "theme", theme: t.theme })} onclick={() => show({ kind: "theme", theme: t.theme })}>{t.theme} <span class="n">{t.count}</span></button>
      {:else}
        <p class="hint">{fr.collectionNoTheme}</p>
      {/each}
      <button class:on={isView({ kind: "none" })} onclick={() => show({ kind: "none" })}>{fr.collectionUntagged} <span class="n">{untagged}</span></button>
      <h3>{fr.collectionRemovedTitle}</h3>
      <button class:on={isView({ kind: "removed" })} onclick={() => show({ kind: "removed" })}>{fr.collectionRemoved} <span class="n">{myImages.removed.length}</span></button>
    </aside>

    <section aria-label={fr.collectionTitle}>
      {#if view.kind !== "removed"}
        <div class="toolbar" class:active={selected.length > 0}>
          {#if selected.length}
            <span><b>{fr.collectionChosen(selected.length)}</b></span>
            <label>
              {fr.collectionFileIn}
              <input list="collection-themes" placeholder={fr.collectionThemePlaceholder} bind:value={themeName} onkeydown={(e) => e.key === "Enter" && file()} />
            </label>
            <datalist id="collection-themes">{#each themes as t (t.theme)}<option value={t.theme}></option>{/each}</datalist>
            <button class="primary" disabled={!themeName.trim()} onclick={file}>{fr.collectionFile}</button>
            {#if view.kind === "theme"}{@const th = view.theme}<button onclick={() => takeOut(selected, th)}>{fr.collectionTakeOut(th)}</button>{/if}
            <button onclick={removeChosen}>{fr.myImagesRemove}</button>
            <button class="plain" onclick={() => (selected = [])}>{fr.collectionUnselect}</button>
          {:else}
            <span class="hint">{fr.collectionSelectHint}</span>
            {#if shown.length}<button class="plain" onclick={() => (selected = shown.map((i) => i.name))}>{fr.librarySelectAll}</button>{/if}
          {/if}
        </div>
      {:else}
        <p class="hint">{fr.collectionRemovedHint}</p>
      {/if}

      {#if shown.length === 0}
        <p class="hint">{view.kind === "removed" ? fr.collectionRemovedEmpty : myImages.images.length ? fr.collectionNoneHere : fr.myImagesEmpty}</p>
      {/if}
      <div class="grid">
        {#each shown as img (img.name)}
          {@const chosen = selected.includes(img.name)}
          <div class="card" class:chosen>
            {#if view.kind === "removed"}
              <div class="pic"><img src={`${removedImageUrl(img.name)}?v=${img.modified}`} alt="" loading="lazy" /></div>
              <span class="name">{displayName(img.name)}</span>
              <button onclick={() => run(() => myImages.restore(img.name))}>↩ {fr.collectionRestore}</button>
            {:else}
              <button class="pic" aria-pressed={chosen} title={fr.collectionChooseHint} onclick={() => toggle(img.name)}>
                <img src={`${myImageUrl(img.name)}?v=${img.modified}`} alt="" loading="lazy" />
                {#if chosen}<span class="tick" aria-hidden="true">✓</span>{/if}
              </button>
              <input class="name-input" value={displayName(img.name)} aria-label={fr.collectionRenameLabel} title={fr.collectionRenameLabel} onchange={(e) => rename(img.name, e.currentTarget)} onkeydown={(e) => e.key === "Enter" && e.currentTarget.blur()} />
              <div class="themes">
                {#each img.themes ?? [] as t (t)}
                  <span class="theme">{t}<button title={fr.collectionTakeOut(t)} aria-label={fr.collectionTakeOut(t)} onclick={() => takeOut([img.name], t)}>×</button></span>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </section>
  </div>
</div>

<style>
  .collection-page {
    position: fixed;
    inset: 0;
    background: var(--bg);
    padding: 24px clamp(16px, 3vw, 40px);
    overflow: auto;
    z-index: 5100;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  h1 {
    font: 600 24px var(--ui-font);
    margin: 0;
    flex: 1;
  }
  .intro {
    margin: 6px 0 14px;
    color: var(--muted);
  }
  h3 {
    margin: 14px 0 6px;
    font-size: 13px;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .back {
    border-color: transparent;
    background: transparent;
  }
  .layout {
    display: grid;
    grid-template-columns: 200px 1fr;
    gap: 16px;
    align-items: start;
  }
  .filters {
    display: flex;
    flex-direction: column;
    gap: 4px;
    position: sticky;
    top: 0;
  }
  .filters > button {
    display: flex;
    justify-content: space-between;
    text-align: left;
    border-color: transparent;
    background: transparent;
  }
  .filters .on {
    background: #e8f1fb;
    border-color: var(--accent);
  }
  .n {
    color: var(--muted);
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    min-height: 40px;
    padding: 4px 8px;
    margin-bottom: 10px;
    border-radius: 8px;
  }
  .toolbar.active {
    background: #e8f1fb;
  }
  .toolbar label {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .primary {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
  .plain {
    border-color: transparent;
    background: transparent;
    text-decoration: underline;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 12px;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px;
    background: white;
    border: 2px solid var(--line);
    border-radius: 10px;
  }
  .card.chosen {
    border-color: var(--accent);
    background: #f3f8fe;
  }
  .pic {
    position: relative;
    height: 110px;
    padding: 4px;
    border: 0;
    background: transparent;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .pic img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }
  .tick {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--accent);
    color: white;
    line-height: 22px;
    text-align: center;
  }
  .name-input {
    font-size: 13px;
    padding: 3px 5px;
    border: 1px solid transparent;
    background: transparent;
  }
  .name-input:hover,
  .name-input:focus {
    border-color: var(--line);
    background: white;
  }
  .name {
    font-size: 13px;
  }
  .themes {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .theme {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 1px 2px 1px 8px;
    border-radius: 10px;
    background: #fff6e6;
    border: 1px solid #f0c98a;
    font-size: 12px;
  }
  .theme button {
    border: 0;
    background: transparent;
    padding: 0 4px;
    font-size: 13px;
    line-height: 1;
  }
  .theme button:hover {
    color: #d32f2f;
  }
  .hint {
    color: var(--muted);
    font-size: 13px;
    margin: 0;
  }
  .error {
    color: #c0392b;
  }
</style>
