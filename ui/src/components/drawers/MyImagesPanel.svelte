<script lang="ts">
  import Icon from "../Icon.svelte";
  import GuideLink from "../GuideLink.svelte";
  // La collection : les images de l'enseignant, gardées d'un document à l'autre. `query` filtre ; pendant une
  // recherche (`searching`), seules les vignettes trouvées sont montrées. Avec `onpick` (choix des motifs d'une
  // frise), un clic ajoute l'image aux motifs ou l'en retire, et `chosen` numérote les motifs choisis.
  import { open } from "@tauri-apps/plugin-dialog";
  import { ask } from "../../lib/ask";
  import { myImageUrl } from "../../lib/api";
  import { editor } from "../../lib/docStore.svelte";
  import { MY_IMAGE_MIME } from "../../lib/dnd";
  import { usePhoto } from "../../lib/editActions";
  import { fr } from "../../lib/i18n/fr";
  import { displayName, filterByTheme, searchMyImages, themeCounts, type ThemeFilter } from "../../lib/myImages";
  import { myImages } from "../../lib/myImages.svelte";
  import { mineMotif } from "../../lib/friezeMotifs.svelte";

  let {
    query = "",
    searching = false,
    onphotos,
    onpick,
    chosen = [],
  }: { query?: string; searching?: boolean; onphotos?: () => void; onpick?: (key: string) => void; chosen?: string[] } = $props();

  myImages.load();

  let filter = $state<ThemeFilter>({ kind: "all" });
  const themes = $derived(themeCounts(myImages.images));
  const shown = $derived(searching ? searchMyImages(myImages.images, query) : filterByTheme(searchMyImages(myImages.images, query), filter));
  const isFilter = (f: ThemeFilter) => JSON.stringify(f) === JSON.stringify(filter);

  async function add() {
    const picked = await open({ multiple: true, filters: [{ name: fr.myImages, extensions: ["png", "jpg", "jpeg", "webp", "svg", "PNG", "JPG", "JPEG", "WEBP", "SVG"] }] });
    const paths = picked === null ? [] : Array.isArray(picked) ? picked : [picked];
    const refused = await myImages.add(paths);
    if (refused) editor.notice = fr.myImagesRefused(refused);
  }

  async function use(name: string) {
    if (onpick) {
      onpick(mineMotif(name));
      return;
    }
    try {
      usePhoto(await myImages.use(name));
    } catch {
      editor.notice = fr.myImagesFailed;
      myImages.load();
    }
  }

  async function remove(name: string) {
    if (!(await ask(fr.myImagesRemoveConfirm(displayName(name)), fr.myImagesRemove))) return;
    await myImages.remove(name);
  }
</script>

{#if !searching}
  <p class="what">{fr.myImagesWhat}</p>
  <button class="add" data-guide="collection-add" onclick={add}><Icon name="add" /> {fr.myImagesAdd}</button>
{/if}
{#if myImages.images.length && (!searching || shown.length)}
  {#if searching}<h4>{fr.myImages}</h4>{/if}
  {#if !searching && themes.length}
    <div class="chips">
      <button class:on={isFilter({ kind: "all" })} onclick={() => (filter = { kind: "all" })}>{fr.allPictures}</button>
      {#each themes as t (t.theme)}
        <button class:on={isFilter({ kind: "theme", theme: t.theme })} onclick={() => (filter = { kind: "theme", theme: t.theme })}>{t.theme} ({t.count})</button>
      {/each}
    </div>
  {/if}
  <div class="grid">
    {#each shown as img (img.name)}
      {@const rank = chosen.indexOf(mineMotif(img.name))}
      <div class="cell">
        <button
          class="thumb"
          class:chosen={rank >= 0}
          title={displayName(img.name)}
          draggable={!onpick}
          onclick={() => use(img.name)}
          ondragstart={(e) => {
            e.dataTransfer?.setData(MY_IMAGE_MIME, img.name);
            if (e.dataTransfer) e.dataTransfer.effectAllowed = "copy";
          }}
        >
          <img src={`${myImageUrl(img.name)}?v=${img.modified}`} alt="" loading="lazy" decoding="async" draggable="false" />
          <span class="name">{displayName(img.name)}</span>
          {#if rank >= 0}<span class="rank">{rank + 1}</span>{/if}
        </button>
        {#if !onpick}<button class="remove" title={fr.myImagesRemove} aria-label={`${fr.myImagesRemove} : ${displayName(img.name)}`} onclick={() => remove(img.name)}><Icon name="close" size={14} /></button>{/if}
      </div>
    {:else}
      <p class="hint">{fr.myImagesNone}</p>
    {/each}
  </div>
  {#if !searching && !onpick}<p class="hint">{fr.myImagesHint}</p>{/if}
{:else if !searching}
  <p class="hint">{fr.myImagesEmpty}</p>
{/if}
{#if !searching && !onpick}
  <button class="organize" data-guide="collection-organize" onclick={() => (editor.showCollection = true)}><Icon name="collection" /> {fr.collectionOrganize}</button>
  <GuideLink id="collection" />
  <p class="hint">{fr.myImagesWhere}</p>
  {#if onphotos}<button class="link" onclick={onphotos}>{fr.myImagesPhotosLink}</button>{/if}
{/if}

<style>
  .what {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }
  .add {
    padding: 8px;
    font-size: 14px;
    font-weight: 600;
  }
  /* Ses propres images : plus grandes que les pictos, avec leur nom. */
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(70px, 1fr));
    gap: 6px;
    max-height: 420px;
    overflow-y: auto;
    padding-top: 4px;
  }
  .cell {
    position: relative;
  }
  .thumb {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 4px;
    border: 1px solid var(--line);
    background: white;
  }
  .thumb {
    position: relative;
  }
  .thumb.chosen {
    border-color: var(--accent);
    background: #e8f1fb;
  }
  .rank {
    position: absolute;
    top: 2px;
    right: 2px;
    min-width: 16px;
    height: 16px;
    border-radius: 8px;
    background: var(--accent);
    color: white;
    font-size: 11px;
    line-height: 16px;
    text-align: center;
  }
  .thumb img {
    width: 100%;
    height: 60px;
    object-fit: contain;
    display: block;
  }
  .name {
    font-size: 11px;
    line-height: 1.2;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Toujours visible pour qu'on la trouve, rouge au survol. */
  .remove {
    position: absolute;
    top: -3px;
    right: -3px;
    width: 18px;
    height: 18px;
    padding: 0;
    border-radius: 50%;
    border: 1px solid var(--line);
    background: white;
    color: var(--muted);
    font-size: 12px;
    line-height: 16px;
  }
  .remove:hover,
  .remove:focus-visible {
    background: #d32f2f;
    border-color: #d32f2f;
    color: white;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .chips button {
    border-radius: 12px;
    padding: 2px 9px;
    font-size: 12px;
  }
  .chips .on {
    background: #e8f1fb;
    border-color: var(--accent);
    color: var(--accent);
  }
  h4 {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--muted);
    text-transform: uppercase;
  }
  .link {
    align-self: flex-start;
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    font-size: 12px;
    text-decoration: underline;
  }
  .hint {
    color: var(--muted);
    font-size: 12px;
    margin: 0;
  }
</style>
