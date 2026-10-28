<script lang="ts">
  // Tiroir Décors : images OpenMoji (recherche, catégories), puis cadres et frises.
  import Drawer from "../layout/Drawer.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { PICTO_MIME } from "../../lib/dnd";
  import { fr } from "../../lib/i18n/fr";
  import { PICTO_GROUPS, pictoElement, pictoUrl, pictos, replacePictos, searchPictos } from "../../lib/pictos.svelte";

  let query = $state("");
  let category = $state<string | null>(null);
  /** Dernière image choisie : motif des frises. */
  let chosen = $state("2600");

  pictos.load();

  const shown = $derived.by(() => {
    const found = searchPictos(pictos.index, query);
    return category && !query ? found.filter((p) => p.group === category) : found;
  });

  function use(code: string) {
    chosen = code;
    const images = editor.page.elements.some((e) => editor.selection.includes(e.id) && e.kind.type === "Image");
    if (images) {
      editor.updatePage((p) => replacePictos(p, editor.selection, code));
      return;
    }
    const el = pictoElement(editor.page, code);
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, el] }));
    editor.selection = [el.id];
  }
</script>

<Drawer title={fr.decor}>
  <h3>{fr.pictures}</h3>
  <input class="search" type="search" bind:value={query} placeholder={fr.searchPicture} aria-label={fr.searchPicture} />
  {#if !query}
    <div class="chips">
      <button class:on={category === null} onclick={() => (category = null)}>{fr.allPictures}</button>
      {#each PICTO_GROUPS as g (g)}
        <button class:on={category === g} onclick={() => (category = g)}>{fr.pictureGroups[g]}</button>
      {/each}
    </div>
  {/if}
  <div class="thumbs">
    {#each shown as p (p.code)}
      <button
        class="thumb"
        class:chosen={p.code === chosen}
        title={p.fr[0] ?? p.en[0]}
        draggable="true"
        onclick={() => use(p.code)}
        ondragstart={(e) => {
          chosen = p.code;
          e.dataTransfer?.setData(PICTO_MIME, p.code);
          if (e.dataTransfer) e.dataTransfer.effectAllowed = "copy";
        }}
      >
        <img src={pictoUrl(p.code)} alt="" loading="lazy" decoding="async" draggable="false" />
      </button>
    {:else}
      <p class="hint">{fr.noPictureFound}</p>
    {/each}
  </div>
  <p class="hint">{fr.pictureHint}</p>
</Drawer>

<style>
  h3 {
    margin: 2px 0 0;
  }
  .search {
    padding: 6px 8px;
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
  .on {
    background: #e8f1fb;
    border-color: var(--accent);
    color: var(--accent);
  }
  .thumbs {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(44px, 1fr));
    gap: 4px;
    max-height: 340px;
    overflow-y: auto;
  }
  .thumb {
    padding: 2px;
    border: 1px solid transparent;
    background: white;
    aspect-ratio: 1;
    /* Les vignettes hors de vue ne sont ni chargées ni peintes. */
    content-visibility: auto;
    contain-intrinsic-size: 44px 44px;
  }
  .thumb.chosen {
    border-color: var(--accent);
  }
  .thumb img {
    width: 100%;
    height: 100%;
    display: block;
  }
  .hint {
    color: var(--muted);
    font-size: 12px;
    margin: 0;
  }
</style>
