<script lang="ts" module>
  // Motifs de la frise : gardés quand le tiroir se ferme et se rouvre.
  let motifs = $state<string[]>(["2600"]);
</script>

<script lang="ts">
  // Tiroir Décors : images OpenMoji (recherche, catégories), puis cadres et frises. En mode « Choisir les
  // images », un clic sur une image l'ajoute aux motifs de la frise (ou l'en retire) sans rien poser.
  import Drawer from "../layout/Drawer.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { PICTO_MIME } from "../../lib/dnd";
  import { fr } from "../../lib/i18n/fr";
  import { PICTO_GROUPS, pictoElement, pictoUrl, pictos, replacePictos, searchPictos } from "../../lib/pictos.svelte";
  import { frieze, pageFrame, toggleMotif, type FrameStyle, type FriezeSide } from "../../lib/decor";
  import type { Element } from "../../lib/types";

  const frames: { style: FrameStyle; label: string }[] = [
    { style: "thin", label: fr.frameThin },
    { style: "thick", label: fr.frameThick },
    { style: "rounded", label: fr.frameRounded },
    { style: "double", label: fr.frameDouble },
  ];
  const sides: { side: FriezeSide; label: string }[] = [
    { side: "top", label: fr.friezeTop },
    { side: "bottom", label: fr.friezeBottom },
    { side: "around", label: fr.friezeAround },
  ];

  /** Cadre : derrière tout le reste ; frise : devant. Les nouveaux éléments sont sélectionnés. */
  function addDecor(els: Element[], behind: boolean) {
    editor.updatePage((p) => ({ ...p, elements: behind ? [...els, ...p.elements] : [...p.elements, ...els] }));
    editor.selection = els.map((e) => e.id);
  }

  let query = $state("");
  let category = $state<string | null>(null);
  let picking = $state(false);

  pictos.load();

  const shown = $derived.by(() => {
    const found = searchPictos(pictos.index, query);
    return category && !query ? found.filter((p) => p.group === category) : found;
  });

  function use(code: string) {
    if (picking) {
      motifs = toggleMotif(motifs, code);
      return;
    }
    const images = editor.page.elements.some((e) => editor.selection.includes(e.id) && e.kind.type === "Image");
    if (images) {
      // Après un double-clic, seule l'image modifiée seule est remplacée ; sinon toute la frise.
      editor.updatePage((p) => replacePictos(p, editor.selection, code, !editor.focusedMember));
      return;
    }
    const el = pictoElement(editor.page, code);
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, el] }));
    editor.selection = [el.id];
  }
</script>

<Drawer title={fr.decor}>
  <h3>{fr.pictures}</h3>
  {#if picking}<p class="picking">{fr.friezePickHint}</p>{/if}
  <input class="search" type="search" bind:value={query} placeholder={fr.searchPicture} aria-label={fr.searchPicture} />
  {#if !query}
    <div class="chips">
      <button class:on={category === null} onclick={() => (category = null)}>{fr.allPictures}</button>
      {#each PICTO_GROUPS as g (g)}
        <button class:on={category === g} onclick={() => (category = g)}>{fr.pictureGroups[g]}</button>
      {/each}
    </div>
  {/if}
  <div class="thumbs" class:picking>
    {#each shown as p (p.code)}
      {@const rank = picking ? motifs.indexOf(p.code) : -1}
      <button
        class="thumb"
        class:chosen={rank >= 0}
        title={p.fr[0] ?? p.en[0]}
        draggable="true"
        onclick={() => use(p.code)}
        ondragstart={(e) => {
          e.dataTransfer?.setData(PICTO_MIME, p.code);
          if (e.dataTransfer) e.dataTransfer.effectAllowed = "copy";
        }}
      >
        <img src={pictoUrl(p.code)} alt="" loading="lazy" decoding="async" draggable="false" />
        {#if rank >= 0}<span class="rank">{rank + 1}</span>{/if}
      </button>
    {:else}
      <p class="hint">{fr.noPictureFound}</p>
    {/each}
  </div>
  {#if !picking}<p class="hint">{fr.pictureHint}</p>{/if}

  <h3>{fr.frames}</h3>
  <div class="frames">
    {#each frames as f (f.style)}
      <button onclick={() => addDecor(pageFrame(editor.page, f.style), true)}><i class="mini {f.style}"></i>{f.label}</button>
    {/each}
  </div>

  <h3>{fr.friezes}</h3>
  <span class="label">{fr.friezeMotifs}</span>
  <div class="motifs">
    {#each motifs as code, i (code)}
      <span class="motif">
        <img src={pictoUrl(code)} alt="" />
        <button class="remove" title={fr.removeMotif} aria-label={fr.removeMotif} onclick={() => (motifs = motifs.filter((_, j) => j !== i))}>×</button>
      </span>
    {:else}
      <span class="hint">{fr.friezeNoMotif}</span>
    {/each}
  </div>
  <button class:on={picking} aria-pressed={picking} onclick={() => (picking = !picking)}>{picking ? `✓ ${fr.friezePickDone}` : `＋ ${fr.friezePick}`}</button>
  <div class="sides">
    {#each sides as s (s.side)}
      <button disabled={motifs.length === 0} onclick={() => addDecor(frieze(editor.page, motifs, s.side), false)}>{s.label}</button>
    {/each}
  </div>
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
  .thumb {
    position: relative;
  }
  .thumb.chosen {
    border-color: var(--accent);
    background: #e8f1fb;
  }
  .rank {
    position: absolute;
    top: 1px;
    right: 1px;
    min-width: 14px;
    height: 14px;
    border-radius: 7px;
    background: var(--accent);
    color: white;
    font-size: 10px;
    line-height: 14px;
    text-align: center;
  }
  .picking {
    margin: 0;
    padding: 6px 8px;
    border-radius: 6px;
    background: #e8f1fb;
    color: var(--accent);
    font-size: 12px;
  }
  .thumbs.picking {
    outline: 2px solid #e8f1fb;
    border-radius: 6px;
  }
  .thumb img {
    width: 100%;
    height: 100%;
    display: block;
  }
  .frames {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 4px;
  }
  .frames button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 6px 2px;
    font-size: 11px;
  }
  .mini {
    display: block;
    width: 26px;
    height: 36px;
    border: 1px solid #555;
    box-sizing: border-box;
  }
  .mini.thick {
    border-width: 3px;
  }
  .mini.rounded {
    border-radius: 5px;
    border-width: 2px;
  }
  .mini.double {
    border-style: double;
    border-width: 4px;
  }
  .label {
    font-size: 12px;
    color: var(--muted);
  }
  .motifs {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    min-height: 34px;
  }
  .motif {
    position: relative;
  }
  .motif img {
    width: 32px;
    height: 32px;
    display: block;
  }
  .remove {
    position: absolute;
    top: -6px;
    right: -6px;
    width: 16px;
    height: 16px;
    padding: 0;
    border-radius: 50%;
    font-size: 11px;
    line-height: 14px;
  }
  .sides {
    display: flex;
    gap: 4px;
  }
  .sides button {
    flex: 1;
    padding: 6px 2px;
    font-size: 12px;
  }
  .hint {
    color: var(--muted);
    font-size: 12px;
    margin: 0;
  }
</style>
