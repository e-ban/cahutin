<script lang="ts">
  // « Mes images » : les images de l'enseignant, gardées d'un document à l'autre. `query` filtre ; pendant une
  // recherche (`searching`), seules les vignettes trouvées sont montrées.
  import { open } from "@tauri-apps/plugin-dialog";
  import { ask } from "../../lib/ask";
  import { myImageUrl } from "../../lib/api";
  import { editor } from "../../lib/docStore.svelte";
  import { MY_IMAGE_MIME } from "../../lib/dnd";
  import { usePhoto } from "../../lib/editActions";
  import { fr } from "../../lib/i18n/fr";
  import { displayName, searchMyImages } from "../../lib/myImages";
  import { myImages } from "../../lib/myImages.svelte";

  let { query = "", searching = false, onphotos }: { query?: string; searching?: boolean; onphotos?: () => void } = $props();

  myImages.load();

  const shown = $derived(searchMyImages(myImages.images, query));

  async function add() {
    const picked = await open({ multiple: true, filters: [{ name: fr.myImages, extensions: ["png", "jpg", "jpeg", "webp", "svg", "PNG", "JPG", "JPEG", "WEBP", "SVG"] }] });
    const paths = picked === null ? [] : Array.isArray(picked) ? picked : [picked];
    const refused = await myImages.add(paths);
    if (refused) editor.notice = fr.myImagesRefused(refused);
  }

  async function use(name: string) {
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
  <button class="add" onclick={add}>＋ {fr.myImagesAdd}</button>
{/if}
{#if myImages.images.length && (!searching || shown.length)}
  {#if searching}<h4>{fr.myImages}</h4>{/if}
  <div class="grid">
    {#each shown as img (img.name)}
      <div class="cell">
        <button
          class="thumb"
          title={displayName(img.name)}
          draggable="true"
          onclick={() => use(img.name)}
          ondragstart={(e) => {
            e.dataTransfer?.setData(MY_IMAGE_MIME, img.name);
            if (e.dataTransfer) e.dataTransfer.effectAllowed = "copy";
          }}
        >
          <img src={`${myImageUrl(img.name)}?v=${img.modified}`} alt="" loading="lazy" decoding="async" draggable="false" />
          <span class="name">{displayName(img.name)}</span>
        </button>
        <button class="remove" title={fr.myImagesRemove} aria-label={`${fr.myImagesRemove} : ${displayName(img.name)}`} onclick={() => remove(img.name)}>×</button>
      </div>
    {:else}
      <p class="hint">{fr.myImagesNone}</p>
    {/each}
  </div>
  {#if !searching}<p class="hint">{fr.myImagesHint}</p>{/if}
{:else if !searching}
  <p class="hint">{fr.myImagesEmpty}</p>
{/if}
{#if !searching}
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
