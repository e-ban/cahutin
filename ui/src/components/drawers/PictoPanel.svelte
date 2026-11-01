<script lang="ts">
  // Pictogrammes OpenMoji : catégories, à colorier, couleur de peau, vignettes. Dans le tiroir Images, un clic
  // pose l'image (ou remplace après « Remplacer ») ; avec `onpick` (choix des motifs d'une frise), un clic
  // ajoute l'image aux motifs ou l'en retire, et `chosen` numérote les motifs choisis.
  import { editor } from "../../lib/docStore.svelte";
  import { PICTO_MIME } from "../../lib/dnd";
  import { fr } from "../../lib/i18n/fr";
  import { CLASS_PICTO_GROUPS, PICTO_GROUPS, SKIN_TONES, pictoElement, pictoUrl, pictos, pictosOf, replacePictos, searchPictos, toned } from "../../lib/pictos.svelte";
  import { isTableCell } from "../../lib/table";

  let { query = "", onpick, chosen = [] }: { query?: string; onpick?: (code: string) => void; chosen?: string[] } = $props();

  let category = $state<string | null>(null);

  pictos.load();

  const shown = $derived(category && !query ? pictosOf(pictos.index, category) : searchPictos(pictos.index, query));

  function use(code: string) {
    if (onpick) {
      onpick(code);
      return;
    }
    // Remplacer seulement sur demande (« Remplacer »), ou pour l'image d'une case de tableau ; sinon un clic pose une
    // image de plus, même si une autre est choisie.
    const selected = editor.page.elements.filter((e) => editor.selection.includes(e.id) && e.kind.type === "Image");
    if (selected.length && (editor.replacingPictos || selected.some((e) => isTableCell(editor.page, e.id)))) {
      // Après un double-clic, seule l'image modifiée seule est remplacée ; sinon toute la frise.
      editor.updatePage((p) => replacePictos(p, editor.selection, code, !editor.focusedMember, pictos.outline));
      return;
    }
    // Dans un modèle, une image posée est fixe (un logo, un dessin), ce qui se change dans ses réglages.
    const el = { ...pictoElement(editor.page, code, undefined, undefined, pictos.outline), locked: editor.templateMode };
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, el] }));
    editor.selection = [el.id];
  }
</script>

{#if !onpick && editor.replacingPictos}<p class="picking">{fr.pictureReplacing}</p>{/if}
{#if !query}
  <div class="chips">
    <button class:on={category === null} onclick={() => (category = null)}>{fr.allPictures}</button>
    {#each Object.keys(CLASS_PICTO_GROUPS) as g (g)}
      <button class="classe" class:on={category === g} onclick={() => (category = g)}>{fr.pictureGroups[g]}</button>
    {/each}
    {#each PICTO_GROUPS as g (g)}
      <button class:on={category === g} onclick={() => (category = g)}>{fr.pictureGroups[g]}</button>
    {/each}
  </div>
{/if}
{#if !onpick}
  <label class="colour-in" title={fr.pictoOutlineHint}><input type="checkbox" checked={pictos.outline} onchange={(e) => pictos.setOutline(e.currentTarget.checked)} /> {fr.pictoOutline}</label>
{/if}
<div class="tones" role="radiogroup" aria-label={fr.skinTone}>
  <span class="label">{fr.skinTone}</span>
  {#each SKIN_TONES as color, i (color)}
    <button class="tone" class:on={pictos.tone === i} role="radio" aria-checked={pictos.tone === i} aria-label={fr.skinTones[i]} title={fr.skinTones[i]} style:background={color} onclick={() => pictos.setTone(i)}></button>
  {/each}
</div>
<div class="thumbs" class:tall={!!onpick}>
  {#each shown as entry (entry.code)}
    {@const p = { ...entry, code: toned(pictos.tones, entry.code, pictos.tone) }}
    {@const rank = chosen.indexOf(p.code)}
    <button
      class="thumb"
      class:chosen={rank >= 0}
      title={p.fr[0] ?? p.en[0]}
      draggable={!onpick}
      onclick={() => use(p.code)}
      ondragstart={(e) => {
        e.dataTransfer?.setData(PICTO_MIME, p.code);
        if (e.dataTransfer) e.dataTransfer.effectAllowed = "copy";
      }}
    >
      <img src={pictoUrl(p.code, !onpick && pictos.outline)} alt="" loading="lazy" decoding="async" draggable="false" />
      {#if rank >= 0}<span class="rank">{rank + 1}</span>{/if}
    </button>
  {:else}
    <p class="hint">{fr.noPictureFound}</p>
  {/each}
</div>
{#if !onpick}<p class="hint">{fr.pictureHint}</p>{/if}

<style>
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
  /* Catégories de la classe, en tête et colorées. */
  .chips .classe {
    border-color: #f0c98a;
    background: #fff6e6;
  }
  .on {
    background: #e8f1fb;
    border-color: var(--accent);
    color: var(--accent);
  }
  .colour-in {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
  }
  .tones {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .tone {
    width: 20px;
    height: 20px;
    padding: 0;
    border-radius: 50%;
    border: 2px solid transparent;
    box-shadow: 0 0 0 1px var(--line);
  }
  .tone.on {
    border-color: var(--accent);
  }
  .thumbs {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(44px, 1fr));
    grid-auto-rows: 44px;
    gap: 4px;
    max-height: 340px;
    overflow-y: auto;
  }
  .thumbs.tall {
    max-height: none;
  }
  .thumb {
    position: relative;
    padding: 2px;
    border: 1px solid transparent;
    background: white;
    /* Pas de content-visibility : le WebKit de l'AppImage (2.50) ne peignait plus les vignettes. Les
       images hors de vue restent non chargées (loading="lazy"). */
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
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    display: block;
  }
  .label {
    font-size: 12px;
    color: var(--muted);
  }
  .hint {
    color: var(--muted);
    font-size: 12px;
    margin: 0;
  }
</style>
