<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { assetUrl, importPhotos, listImportableIn } from "../lib/api";
  import { editor } from "../lib/docStore.svelte";
  import { centeredFrame, newId, textElement } from "../lib/factory";
  import { fr } from "../lib/i18n/fr";
  import { ASSET_MIME } from "../lib/dnd";
  import { summarizeImport } from "../lib/importSummary";
  import type { ShapeKind } from "../lib/types";

  const EXTENSIONS = ["jpg", "jpeg", "png", "webp", "heic", "heif", "JPG", "JPEG", "PNG", "WEBP", "HEIC", "HEIF"];

  let busy = $state(false);

  async function importPaths(paths: string[]) {
    if (paths.length === 0) return;
    busy = true;
    try {
      const { imported, message } = summarizeImport(await importPhotos(paths));
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
    if (typeof dir === "string") await importPaths(await listImportableIn(dir));
  }

  function addShape(shape: ShapeKind) {
    const frame = centeredFrame(editor.page, 60, 40);
    const id = newId();
    editor.updatePage((p) => ({
      ...p,
      elements: [
        ...p.elements,
        { id, frame, locked: false, zone: false, kind: { type: "Shape", shape, fill: "#f6e3c5", stroke: null, corner_radius_mm: 0 } },
      ],
    }));
    editor.selection = [id];
  }

  function addText() {
    const el = textElement(centeredFrame(editor.page, 80, 20));
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, el] }));
    editor.selection = [el.id];
    editor.editingText = el.id;
  }
</script>

<aside class="left">
  <section>
    <h3>{fr.photos}</h3>
    <div class="row">
      <button onclick={pickFiles} disabled={busy}>🖼 {fr.addPhotos}</button>
      <button onclick={pickFolder} disabled={busy}>📁 {fr.addFolder}</button>
      <button onclick={pickFiles} disabled={busy}>⭐ {fr.images}</button>
    </div>
    {#if busy}<p class="hint">Import en cours…</p>{/if}
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
      <p class="hint">Glissez une photo sur une zone de la page.</p>
    {/if}
  </section>
  <section>
    <h3>{fr.text}</h3>
    <button onclick={addText}>✏️ {fr.addText}</button>
  </section>
  <section>
    <h3>{fr.shapes}</h3>
    <div class="row">
      <button onclick={() => addShape("Rect")}>▭ {fr.rect}</button>
      <button onclick={() => addShape("Ellipse")}>◯ {fr.ellipse}</button>
    </div>
  </section>
</aside>

<style>
  .left {
    grid-area: left;
    background: var(--panel);
    border-right: 1px solid var(--line);
    overflow-y: auto;
    padding: 12px;
  }
  section {
    margin-bottom: 18px;
  }
  .row {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .thumbs {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
    margin-top: 10px;
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
  }
</style>
