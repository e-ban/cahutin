<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import Drawer from "../layout/Drawer.svelte";
  import { assetUrl, listImportableIn } from "../../lib/api";
  import { editor } from "../../lib/docStore.svelte";
  import { ASSET_MIME } from "../../lib/dnd";
  import { usePhoto } from "../../lib/editActions";
  import { importPaths } from "../../lib/fileActions";
  import { fr } from "../../lib/i18n/fr";
  import { filterPhotos, folderChips, usedAssetIds, type PhotoFilter } from "../../lib/photoLibrary";
  import { readPref, writePref } from "../../lib/prefs";

  const EXTENSIONS = ["jpg", "jpeg", "png", "webp", "heic", "heif", "JPG", "JPEG", "PNG", "WEBP", "HEIC", "HEIF"];
  const SIZES = [
    { px: 72, label: fr.thumbSmall, icon: "▪" },
    { px: 96, label: fr.thumbMedium, icon: "◼" },
    { px: 140, label: fr.thumbLarge, icon: "⬛" },
  ];

  let filter = $state<PhotoFilter>({ kind: "all" });
  let thumbSize = $state(readPref("thumbSize", 96));

  const used = $derived(usedAssetIds(editor.doc));
  const chips = $derived(folderChips(editor.photos));
  const shown = $derived(filterPhotos(editor.photos, filter, used));
  const unusedCount = $derived(editor.photos.filter((p) => !used.has(p.meta.id)).length);

  const isFilter = (f: PhotoFilter) => JSON.stringify(f) === JSON.stringify(filter);

  function setSize(px: number) {
    thumbSize = px;
    writePref("thumbSize", px);
  }

  async function pickFiles() {
    const picked = await open({ multiple: true, filters: [{ name: "Images", extensions: EXTENSIONS }] });
    if (picked) await importPaths(Array.isArray(picked) ? picked : [picked]);
  }

  async function pickFolder() {
    const dir = await open({ directory: true });
    if (typeof dir === "string") await importPaths(await listImportableIn(dir), { fromFolder: true });
  }
</script>

<Drawer title={fr.photos}>
  <div class="row">
    <button class="add" onclick={pickFiles}>{fr.addPhotosButton}</button>
    <button class="add" onclick={pickFolder}>{fr.addFolderButton}</button>
  </div>

  {#if editor.importProgress}
    {@const p = editor.importProgress}
    <div class="progress" role="status">
      <span>{fr.importProgress(p.done, p.total)}</span>
      <progress max={p.total} value={p.done}></progress>
    </div>
  {/if}

  {#if editor.photos.length === 0}
    <p class="hint">{fr.noPhotosYet}</p>
  {:else}
    <div class="chips">
      <button class:on={isFilter({ kind: "all" })} onclick={() => (filter = { kind: "all" })}>{fr.filterAll(editor.photos.length)}</button>
      <button class:on={isFilter({ kind: "unused" })} onclick={() => (filter = { kind: "unused" })}>{fr.filterUnused(unusedCount)}</button>
      {#each chips as c (c.folder)}
        <button class:on={isFilter({ kind: "folder", folder: c.folder })} onclick={() => (filter = { kind: "folder", folder: c.folder })}>📁 {c.folder} ({c.count})</button>
      {/each}
    </div>
    <div class="sizes" role="group" aria-label={fr.thumbSize}>
      {#each SIZES as s (s.px)}
        <button class:on={thumbSize === s.px} title={s.label} aria-label={s.label} onclick={() => setSize(s.px)}>{s.icon}</button>
      {/each}
    </div>
    <div class="thumbs" style:--thumb="{thumbSize}px">
      {#each shown as p (p.meta.id)}
        <button
          class="thumb"
          draggable="true"
          title={used.has(p.meta.id) ? fr.usedInDocument : undefined}
          onclick={() => usePhoto(p.meta)}
          ondragstart={(e) => {
            e.dataTransfer?.setData(ASSET_MIME, p.meta.id);
            if (e.dataTransfer) e.dataTransfer.effectAllowed = "copy";
          }}
        >
          <img src={assetUrl(p.meta.id, true)} alt="" loading="lazy" decoding="async" draggable="false" data-asset={p.meta.id} />
          {#if used.has(p.meta.id)}<span class="check" aria-label={fr.usedInDocument}>✓</span>{/if}
        </button>
      {/each}
    </div>
    <p class="hint">{fr.photoClickHint}</p>
  {/if}
</Drawer>

<style>
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
    background: #e8f1fb;
    border-color: var(--accent);
    color: var(--accent);
  }
  .sizes {
    display: flex;
    gap: 2px;
    justify-content: flex-end;
  }
  .sizes button {
    padding: 0 7px;
    font-size: 11px;
  }
  .thumbs {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--thumb), 1fr));
    gap: 6px;
  }
  .thumb {
    position: relative;
    padding: 0;
    border: 0;
    border-radius: 4px;
    overflow: hidden;
    aspect-ratio: 1;
    background: #eee;
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
  .check {
    position: absolute;
    right: 4px;
    bottom: 4px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #27ae60;
    color: white;
    font-size: 12px;
    line-height: 18px;
    text-align: center;
    box-shadow: 0 0 0 2px white;
  }
  .hint {
    color: var(--muted);
    font-size: 12px;
    margin: 0;
  }
</style>
