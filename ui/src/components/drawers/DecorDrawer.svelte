<script lang="ts">
  // Tiroir Décors : images OpenMoji (recherche, catégories), puis cadres et frises. En mode « Choisir les
  // images », un clic sur une image l'ajoute aux motifs de la frise (ou l'en retire) sans rien poser.
  // « Mes images » : les images de l'enseignant, gardées d'un document à l'autre.
  import { onDestroy } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { ask } from "../../lib/ask";
  import { myImageUrl } from "../../lib/api";
  import { usePhoto } from "../../lib/editActions";
  import { displayName, searchMyImages } from "../../lib/myImages";
  import { myImages } from "../../lib/myImages.svelte";
  import Drawer from "../layout/Drawer.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { MY_IMAGE_MIME, PICTO_MIME } from "../../lib/dnd";
  import { fr } from "../../lib/i18n/fr";
  import { CLASS_PICTO_GROUPS, PICTO_GROUPS, SKIN_TONES, pictoElement, pictoUrl, pictos, pictosOf, replacePictos, searchPictos, toned } from "../../lib/pictos.svelte";
  import { frieze, pageFrame, toggleMotif, type FrameStyle, type FriezeSide } from "../../lib/decor";
  import { friezeMotifs } from "../../lib/friezeMotifs.svelte";
  import { isTableCell } from "../../lib/table";
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

  // Le mode « Remplacer » prend fin quand on choisit autre chose sur la page, ou qu'on ferme le tiroir.
  onDestroy(() => (editor.replacingPictos = false));
  let selectionWhenOpened = editor.selection;
  $effect.pre(() => {
    if (editor.selection !== selectionWhenOpened) {
      selectionWhenOpened = editor.selection;
      editor.replacingPictos = false;
    }
  });

  /** Catégorie « Mes images » : à part des images fournies. */
  const MINE = "mine";
  let query = $state("");
  let category = $state<string | null>(null);
  let picking = $state(false);

  pictos.load();
  myImages.load();

  // Pendant le choix des motifs d'une frise ou un « Remplacer », seules les images fournies comptent.
  const mine = $derived(category === MINE && !picking && !editor.replacingPictos);
  const shownMine = $derived(searchMyImages(myImages.images, query));

  async function addMine() {
    const picked = await open({ multiple: true, filters: [{ name: fr.myImages, extensions: ["png", "jpg", "jpeg", "webp", "svg", "PNG", "JPG", "JPEG", "WEBP", "SVG"] }] });
    const paths = picked === null ? [] : Array.isArray(picked) ? picked : [picked];
    const refused = await myImages.add(paths);
    if (refused) editor.notice = fr.myImagesRefused(refused);
  }

  async function useMine(name: string) {
    try {
      usePhoto(await myImages.use(name));
    } catch {
      editor.notice = fr.myImagesFailed;
      myImages.load();
    }
  }

  async function removeMine(name: string) {
    if (!(await ask(fr.myImagesRemoveConfirm(displayName(name)), fr.myImagesRemove))) return;
    await myImages.remove(name);
  }

  const shown = $derived.by(() => {
    const found = searchPictos(pictos.index, query);
    return category && category !== MINE && !query ? pictosOf(pictos.index, category) : found;
  });

  function use(code: string) {
    if (picking) {
      friezeMotifs.codes = toggleMotif(friezeMotifs.codes, code);
      return;
    }
    // Remplacer seulement sur demande (« Remplacer »), ou pour l'image d'une case de tableau ; sinon un clic pose une
    // image de plus, même si une autre est choisie.
    const chosen = editor.page.elements.filter((e) => editor.selection.includes(e.id) && e.kind.type === "Image");
    if (chosen.length && (editor.replacingPictos || chosen.some((e) => isTableCell(editor.page, e.id)))) {
      // Après un double-clic, seule l'image modifiée seule est remplacée ; sinon toute la frise.
      editor.updatePage((p) => replacePictos(p, editor.selection, code, !editor.focusedMember, pictos.outline));
      return;
    }
    const el = pictoElement(editor.page, code, undefined, undefined, pictos.outline);
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, el] }));
    editor.selection = [el.id];
  }
</script>

<Drawer title={fr.decor}>
  <h3>{fr.pictures}</h3>
  {#if picking}<p class="picking">{fr.friezePickHint}</p>{:else if editor.replacingPictos}<p class="picking">{fr.pictureReplacing}</p>{/if}
  <input class="search" type="search" bind:value={query} placeholder={fr.searchPicture} aria-label={fr.searchPicture} />
  {#if !query || mine}
    <div class="chips">
      {#if !picking && !editor.replacingPictos}<button class="mine" class:on={category === MINE} onclick={() => (category = MINE)}>{fr.myImages}</button>{/if}
      <button class:on={category === null} onclick={() => (category = null)}>{fr.allPictures}</button>
      {#each Object.keys(CLASS_PICTO_GROUPS) as g (g)}
        <button class="classe" class:on={category === g} onclick={() => (category = g)}>{fr.pictureGroups[g]}</button>
      {/each}
      {#each PICTO_GROUPS as g (g)}
        <button class:on={category === g} onclick={() => (category = g)}>{fr.pictureGroups[g]}</button>
      {/each}
    </div>
  {/if}
  {#if mine}
    <button class="add-mine" onclick={addMine}>＋ {fr.myImagesAdd}</button>
    {#if myImages.images.length}
      <div class="mine-grid">
        {#each shownMine as img (img.name)}
          <div class="mine-thumb">
            <button
              class="thumb"
              title={displayName(img.name)}
              draggable="true"
              onclick={() => useMine(img.name)}
              ondragstart={(e) => {
                e.dataTransfer?.setData(MY_IMAGE_MIME, img.name);
                if (e.dataTransfer) e.dataTransfer.effectAllowed = "copy";
              }}
            >
              <img src={`${myImageUrl(img.name)}?v=${img.modified}`} alt="" loading="lazy" decoding="async" draggable="false" />
              <span class="name">{displayName(img.name)}</span>
            </button>
            <button class="remove" title={fr.myImagesRemove} aria-label={`${fr.myImagesRemove} : ${displayName(img.name)}`} onclick={() => removeMine(img.name)}>×</button>
          </div>
        {:else}
          <p class="hint">{fr.myImagesNone}</p>
        {/each}
      </div>
      <p class="hint">{fr.myImagesHint}</p>
    {:else}
      <p class="hint">{fr.myImagesEmpty}</p>
    {/if}
    <p class="hint">{fr.myImagesWhere}</p>
  {:else}
  <label class="colour-in" title={fr.pictoOutlineHint}><input type="checkbox" checked={pictos.outline} onchange={(e) => pictos.setOutline(e.currentTarget.checked)} /> {fr.pictoOutline}</label>
  <div class="tones" role="radiogroup" aria-label={fr.skinTone}>
    <span class="label">{fr.skinTone}</span>
    {#each SKIN_TONES as color, i (color)}
      <button class="tone" class:on={pictos.tone === i} role="radio" aria-checked={pictos.tone === i} aria-label={fr.skinTones[i]} title={fr.skinTones[i]} style:background={color} onclick={() => pictos.setTone(i)}></button>
    {/each}
  </div>
  <div class="thumbs" class:picking>
    {#each shown as entry (entry.code)}
      {@const p = { ...entry, code: toned(pictos.tones, entry.code, pictos.tone) }}
      {@const rank = picking ? friezeMotifs.codes.indexOf(p.code) : -1}
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
        <img src={pictoUrl(p.code, pictos.outline)} alt="" loading="lazy" decoding="async" draggable="false" />
        {#if rank >= 0}<span class="rank">{rank + 1}</span>{/if}
      </button>
    {:else}
      <p class="hint">{fr.noPictureFound}</p>
    {/each}
  </div>
  {#if !picking}<p class="hint">{fr.pictureHint}</p>{/if}
  {/if}

  <h3>{fr.frames}</h3>
  <div class="frames">
    {#each frames as f (f.style)}
      <button onclick={() => addDecor(pageFrame(editor.page, f.style), true)}><i class="mini {f.style}"></i>{f.label}</button>
    {/each}
  </div>

  <h3>{fr.friezes}</h3>
  <span class="label">{fr.friezeMotifs}</span>
  <div class="motifs">
    {#each friezeMotifs.codes as code, i (code)}
      <span class="motif">
        <img src={pictoUrl(code)} alt="" />
        <button class="remove" title={fr.removeMotif} aria-label={fr.removeMotif} onclick={() => (friezeMotifs.codes = friezeMotifs.codes.filter((_, j) => j !== i))}>×</button>
      </span>
    {:else}
      <span class="hint">{fr.friezeNoMotif}</span>
    {/each}
  </div>
  <button class:on={picking} aria-pressed={picking} onclick={() => (picking = !picking)}>{picking ? `✓ ${fr.friezePickDone}` : `＋ ${fr.friezePick}`}</button>
  <div class="sides">
    {#each sides as s (s.side)}
      <button disabled={friezeMotifs.codes.length === 0} onclick={() => addDecor(frieze(editor.page, friezeMotifs.codes, s.side), false)}>{s.label}</button>
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
  /* Mes images : la première catégorie, bien visible. */
  .chips .mine {
    border-color: #9cc59a;
    background: #eef8ec;
    font-weight: 600;
  }
  .add-mine {
    padding: 8px;
    font-size: 14px;
    font-weight: 600;
  }
  /* Ses propres images : plus grandes que les pictos, avec leur nom. */
  .mine-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(70px, 1fr));
    gap: 6px;
    max-height: 420px;
    overflow-y: auto;
    padding-top: 4px;
  }
  .mine-thumb {
    position: relative;
  }
  .mine-thumb .thumb {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 4px;
    border-color: var(--line);
  }
  .mine-thumb .thumb img {
    height: 60px;
  }
  .name {
    font-size: 11px;
    line-height: 1.2;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mine-thumb .remove {
    top: -3px;
    right: -3px;
    background: white;
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
  .thumb {
    padding: 2px;
    border: 1px solid transparent;
    background: white;
    /* Pas de content-visibility : le WebKit de l'AppImage (2.50) ne peignait plus les vignettes. Les
       images hors de vue restent non chargées (loading="lazy"). */
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
    object-fit: contain;
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
