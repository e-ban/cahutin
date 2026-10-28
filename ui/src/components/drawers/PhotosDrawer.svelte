<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import Drawer from "../layout/Drawer.svelte";
  import { assetUrl, importPhotos, listImportableIn } from "../../lib/api";
  import { editor } from "../../lib/docStore.svelte";
  import { ASSET_MIME } from "../../lib/dnd";
  import { fr } from "../../lib/i18n/fr";
  import { summarizeImport } from "../../lib/importSummary";
  import type { ImportOutcome } from "../../lib/types";

  const EXTENSIONS = ["jpg", "jpeg", "png", "webp", "heic", "heif", "JPG", "JPEG", "PNG", "WEBP", "HEIC", "HEIF"];

  let busy = $state(false);

  async function importPaths(paths: string[], fromFolder = false) {
    if (paths.length === 0 && !fromFolder) return;
    busy = true;
    try {
      const outcomes: ImportOutcome[] = [];
      if (paths.length) await importPhotos(paths, (o) => outcomes.push(o));
      const { imported, message } = summarizeImport(outcomes, { fromFolder });
      editor.addPhotos(imported);
      if (message) editor.notice = message;
    } finally {
      busy = false;
    }
  }

  async function pickFiles() {
    const picked = await open({ multiple: true, filters: [{ name: "Images", extensions: EXTENSIONS }] });
    if (picked) await importPaths(Array.isArray(picked) ? picked : [picked]);
  }

  async function pickFolder() {
    const dir = await open({ directory: true });
    if (typeof dir === "string") await importPaths(await listImportableIn(dir), true);
  }
</script>

<Drawer title={fr.photos}>
  <div class="row">
    <button onclick={pickFiles} disabled={busy}>＋ {fr.photos}</button>
    <button onclick={pickFolder} disabled={busy}>📁 Dossier</button>
  </div>
  {#if busy}<p class="hint">{fr.importing}</p>{/if}
  {#if editor.photos.length > 0}
    <div class="thumbs">
      {#each editor.photos as p (p.id)}
        <img
          src={assetUrl(p.id, true)}
          alt=""
          draggable="true"
          data-asset={p.id}
          ondragstart={(e) => {
            e.dataTransfer?.setData(ASSET_MIME, p.id);
            if (e.dataTransfer) e.dataTransfer.effectAllowed = "copy";
          }}
        />
      {/each}
    </div>
    <p class="hint">{fr.dragPhotoHint}</p>
  {/if}
</Drawer>

<style>
  .row {
    display: flex;
    gap: 6px;
  }
  .thumbs {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
  }
  .thumbs img {
    width: 100%;
    aspect-ratio: 1;
    object-fit: cover;
    border-radius: 4px;
    cursor: grab;
    background: #eee;
  }
  .hint {
    color: var(--muted);
    font-size: 12px;
    margin: 0;
  }
</style>
