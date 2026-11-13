<script lang="ts">
  // Rubrique Photos du tiroir Images : les photos de ce document, et l'accès à la photothèque de la classe.
  // Avec beaucoup de photos, la barre du haut reste en place, les photos sont rangées par jour et le tiroir
  // s'ouvre sur celles qui ne sont pas encore placées.
  import Icon from "../Icon.svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { assetUrl, listImportableIn } from "../../lib/api";
  import { ask } from "../../lib/ask";
  import { editor } from "../../lib/docStore.svelte";
  import { ASSET_MIME } from "../../lib/dnd";
  import { askRemoveUnusedPhotos, usePhoto } from "../../lib/editActions";
  import { importPaths } from "../../lib/fileActions";
  import { fr } from "../../lib/i18n/fr";
  import { MANY_PHOTOS, assetsOnPage, byDay, elementAssets, filterPhotos, folderChips, newlyPlaced, pageBadge, pagesUsing, shownFilter, startFilter, usedAssetIds, type PhotoFilter } from "../../lib/photoLibrary";
  import { readPref, writePref } from "../../lib/prefs";

  const EXTENSIONS = ["jpg", "jpeg", "png", "webp", "svg", "heic", "heif", "JPG", "JPEG", "PNG", "WEBP", "SVG", "HEIC", "HEIF"];
  const SIZES = [
    { px: 72, label: fr.thumbSmall, iconSize: 10 },
    { px: 96, label: fr.thumbMedium, iconSize: 13 },
    { px: 140, label: fr.thumbLarge, iconSize: 16 },
  ];

  let chosen = $state<PhotoFilter>(startFilter(readPref<PhotoFilter | null>("photoFilter", null), editor.photos.length));
  let thumbSize = $state(readPref("thumbSize", 96));

  const used = $derived(usedAssetIds(editor.doc));
  const onPage = $derived(assetsOnPage(editor.page));
  const chips = $derived(folderChips(editor.photos));
  const filter = $derived(shownFilter(chosen, chips));
  const shown = $derived(filterPhotos(editor.photos, filter, used, onPage));
  const days = $derived(byDay(shown));
  const unusedCount = $derived(editor.photos.filter((p) => !used.has(p.meta.id)).length);
  // Le retrait suit le filtre : la vue d'un dossier ne retire que ses photos non placées.
  const removeOnly = $derived(filter.kind === "folder" ? new Set(shown.map((p) => p.meta.id)) : undefined);
  const removeCount = $derived(shown.filter((p) => !used.has(p.meta.id)).length);
  const pageCount = $derived(editor.photos.filter((p) => onPage.has(p.meta.id)).length);
  const many = $derived(editor.photos.length >= MANY_PHOTOS);
  // Les pages de chaque photo posée, calculées une fois pour tout le tiroir.
  const pagesOf = $derived.by(() => {
    const m = new Map<string, number[]>();
    editor.doc.pages.forEach((p, i) => {
      for (const id of new Set(p.elements.flatMap(elementAssets))) m.set(id, [...(m.get(id) ?? []), i + 1]);
    });
    return m;
  });

  const isFilter = (f: PhotoFilter) => JSON.stringify(f) === JSON.stringify(filter);

  /** Un dossier est propre au document : seuls Toutes, Non placées et Sur cette page sont retenus. */
  function setFilter(f: PhotoFilter) {
    chosen = f;
    if (f.kind !== "folder") writePref("photoFilter", f);
  }

  // La première fois qu'une photo du tiroir posée quitte « Non placées », on dit pourquoi (pas quand un autre
  // document s'ouvre, tiroir ouvert).
  let before: { used: Set<string>; drawer: Set<string> } | null = null;
  $effect(() => {
    const now = { used, drawer: new Set(editor.photos.map((p) => p.meta.id)) };
    if (before && filter.kind === "unused" && newlyPlaced(before.used, before.drawer, now.used, now.drawer) && !readPref("unusedHintShown", false)) {
      editor.notice = fr.photosUnusedHint;
      writePref("unusedHintShown", true);
    }
    before = now;
  });

  function setSize(px: number) {
    thumbSize = px;
    writePref("thumbSize", px);
  }

  async function pickFiles() {
    const picked = await open({ multiple: true, filters: [{ name: "Images", extensions: EXTENSIONS }] });
    if (picked) await importPaths(Array.isArray(picked) ? picked : [picked]);
  }

  async function remove(id: string) {
    if (await ask(fr.photoRemoveConfirm(pagesUsing(editor.doc, id)), fr.photoRemove)) editor.removePhoto(id);
  }

  async function pickFolder() {
    const dir = await open({ directory: true });
    if (typeof dir === "string") await importPaths(await listImportableIn(dir), { fromFolder: true });
  }
</script>

<!-- La barre reste en haut quand on fait défiler les photos. -->
<div class="bar">
  {#if !many}<p class="what">{fr.photosWhat}</p>{/if}
  <div class="row">
    <button class="add" onclick={pickFiles}><Icon name="add" /> {fr.addPhotosButton}</button>
    <button class="add" onclick={pickFolder}><Icon name="folder" /> {fr.addFolderButton}</button>
  </div>
  <!-- Les photos de l'année, rangées par enfant et par activité : on y choisit celles du document. -->
  <button class="library" title={fr.libraryOpenHint} onclick={() => (editor.showPhototheque = true)}><Icon name="images" /> {fr.libraryFromDrawer}</button>

  {#if editor.replacingPhoto}<p class="replacing">{fr.photoReplacing}</p>{/if}

  {#if editor.importProgress}
    {@const p = editor.importProgress}
    <div class="progress" role="status">
      <span>{fr.importProgress(p.done, p.total)}</span>
      <progress max={p.total} value={p.done}></progress>
    </div>
  {/if}

  {#if editor.photos.length > 0}
    <div class="chips">
      <button class:on={isFilter({ kind: "all" })} onclick={() => setFilter({ kind: "all" })}>{fr.filterAll(editor.photos.length)}</button>
      <button class:on={isFilter({ kind: "unused" })} onclick={() => setFilter({ kind: "unused" })}>{fr.filterUnused(unusedCount)}</button>
      <button class:on={isFilter({ kind: "page" })} onclick={() => setFilter({ kind: "page" })}>{fr.filterPage(pageCount)}</button>
      {#each chips as c (c.folder)}
        <button class:on={isFilter({ kind: "folder", folder: c.folder })} onclick={() => setFilter({ kind: "folder", folder: c.folder })}><Icon name="folder" size={14} /> {c.folder} ({c.count})</button>
      {/each}
    </div>
    <div class="tools">
      <!-- Dès que la vue montre une photo non placée (Toutes, Non placées, un dossier). Pas pendant un import :
           les photos encore en route reviendraient après le retrait. -->
      {#if shown.some((p) => !used.has(p.meta.id)) && !editor.importProgress}
        <button class="link" onclick={() => askRemoveUnusedPhotos(removeOnly)}>{fr.photosRemoveUnused(removeCount)}</button>
      {/if}
      <div class="sizes" role="group" aria-label={fr.thumbSize}>
        {#each SIZES as s (s.px)}
          <button class:on={thumbSize === s.px} title={s.label} aria-label={s.label} onclick={() => setSize(s.px)}><Icon name="border" size={s.iconSize} /></button>
        {/each}
      </div>
    </div>
  {/if}
</div>

{#if editor.photos.length === 0}
  <p class="hint">{fr.noPhotosYet}</p>
{:else}
  {#if filter.kind === "page" && shown.length === 0}<p class="hint">{fr.photosNoneOnPage}</p>{/if}
  {#if filter.kind === "unused" && shown.length === 0}<p class="hint">{fr.photosAllPlaced}</p>{/if}
  {#each days as d (d.day)}
    <h4 class="day">{d.day ? fr.libraryDayShort(d.day) : fr.photosNoDate} <span>· {d.photos.length}</span></h4>
    <div class="thumbs" style:--thumb="{thumbSize}px">
      {#each d.photos as p (p.meta.id)}
        {@const pages = pagesOf.get(p.meta.id) ?? []}
        {@const badge = pageBadge(pages)}
        <div class="cell">
          <button
            class="thumb"
            draggable="true"
            onclick={() => usePhoto(p.meta)}
            ondragstart={(e) => {
              e.dataTransfer?.setData(ASSET_MIME, p.meta.id);
              if (e.dataTransfer) e.dataTransfer.effectAllowed = "copy";
            }}
          >
            <img src={assetUrl(p.meta.id, true)} alt="" loading="lazy" decoding="async" draggable="false" data-asset={p.meta.id} />
          </button>
          {#if badge}
            <!-- La page où la photo est posée : un clic y mène. -->
            <button class="page" title={fr.photoGoToPage(pages[0])} aria-label={fr.photoGoToPage(pages[0])} onclick={() => editor.goToPage(pages[0])}>{badge}</button>
          {/if}
          <button class="remove" title={fr.photoRemove} aria-label={fr.photoRemove} onclick={() => remove(p.meta.id)}><Icon name="close" size={14} /></button>
        </div>
      {/each}
    </div>
  {/each}
  {#if !many}<p class="hint">{fr.photoClickHint}</p>{/if}
{/if}
{#if !many}<p class="hint">{fr.photosKeepHint}</p>{/if}

<style>
  /* Collée en haut du tiroir, sur son fond : seules les vignettes défilent dessous. */
  .bar {
    position: sticky;
    top: -8px;
    z-index: 2;
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: -8px -12px 0;
    padding: 8px 12px 6px;
    background: var(--panel);
    border-bottom: 1px solid var(--line);
  }
  .what {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }
  .cell {
    position: relative;
  }
  /* Toujours visible pour qu'on la trouve, rouge au survol. */
  .remove {
    position: absolute;
    top: 2px;
    right: 2px;
    width: 20px;
    height: 20px;
    padding: 0;
    border-radius: 50%;
    border: 1px solid var(--line);
    background: color-mix(in srgb, var(--surface) 90%, transparent);
    color: var(--muted);
    font-size: 14px;
    line-height: 18px;
  }
  .remove:hover,
  .remove:focus-visible {
    background: var(--danger);
    border-color: var(--danger);
    color: var(--on-accent);
  }
  /* Le numéro de la page où la photo est posée. */
  .page {
    position: absolute;
    right: 4px;
    bottom: 4px;
    padding: 0 6px;
    border: 0;
    border-radius: 9px;
    background: var(--ok);
    color: var(--on-accent);
    font-size: 11px;
    font-weight: 600;
    line-height: 18px;
    box-shadow: 0 0 0 2px var(--surface);
  }
  .page:hover,
  .page:focus-visible {
    text-decoration: underline;
  }
  .replacing {
    margin: 0;
    padding: 6px 8px;
    border-radius: 6px;
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 12px;
  }
  .library {
    width: 100%;
  }
  .row {
    display: flex;
    gap: 6px;
  }
  .add {
    flex: 1;
  }
  .progress {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--muted);
  }
  progress {
    width: 100%;
    accent-color: var(--accent);
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
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent);
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    font-size: 12px;
    text-decoration: underline;
    text-align: left;
  }
  .sizes {
    display: flex;
    gap: 2px;
    margin-left: auto;
  }
  .sizes button {
    padding: 0 7px;
    font-size: 11px;
  }
  .day {
    margin: 4px 0 0;
    font-size: 12px;
    font-weight: 600;
  }
  .day span {
    color: var(--muted);
    font-weight: 400;
  }
  .thumbs {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--thumb), 1fr));
    gap: 6px;
  }
  .thumb {
    position: relative;
    width: 100%;
    display: block;
    padding: 0;
    border: 0;
    border-radius: 4px;
    overflow: hidden;
    aspect-ratio: 1;
    background: var(--bg);
    cursor: grab;
    /* Les vignettes hors de vue ne sont ni mises en page ni peintes. */
    content-visibility: auto;
    contain-intrinsic-size: var(--thumb) var(--thumb);
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .hint {
    color: var(--muted);
    font-size: 12px;
    margin: 0;
  }
</style>
