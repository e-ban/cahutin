<script lang="ts">
  import "./fonts.css";
  import "./print.css";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import Modal from "./components/Modal.svelte";
  import PrintRoot from "./components/PrintRoot.svelte";
  import { confirmLeave } from "./lib/fileActions";
  import LeftPanel from "./components/LeftPanel.svelte";
  import PageBar from "./components/PageBar.svelte";
  import PageView from "./components/PageView.svelte";
  import RightPanel from "./components/RightPanel.svelte";
  import TemplateBar from "./components/TemplateBar.svelte";
  import TemplatePicker from "./components/TemplatePicker.svelte";
  import Toolbar from "./components/Toolbar.svelte";
  import Transformer from "./components/Transformer.svelte";
  import { editor } from "./lib/docStore.svelte";
  import { fonts } from "./lib/fonts.svelte";
  import { handleShortcut } from "./lib/shortcuts";
  import { newDocument, newId } from "./lib/factory";
  import { templates } from "./lib/templates.svelte";
  import { mmToPx } from "./lib/units";
  import type { Crop, RichText } from "./lib/types";

  editor.reset(newDocument(), null);
  editor.showPicker = true;
  fonts.load();
  templates.refresh();
  if (import.meta.env.DEV) (window as unknown as { __editor: typeof editor }).__editor = editor;

  // Fermeture de la fenêtre : demander d'enregistrer s'il y a des modifications.
  onMount(() => {
    let unlisten: (() => void) | undefined;
    try {
      const win = getCurrentWindow();
      win
        .onCloseRequested(async (event) => {
          if (!editor.dirty) return;
          event.preventDefault();
          if (await confirmLeave()) await win.destroy();
        })
        .then((u) => (unlisten = u))
        .catch(() => {});
    } catch {
      // Hors de l'application (aperçu dans un navigateur) : rien à faire.
    }
    return () => unlisten?.();
  });

  let stageWidth = $state(800);
  let pageEl = $state<HTMLElement>();
  $effect(() => {
    editor.fitZoom = Math.max(0.2, (stageWidth - 64) / mmToPx(editor.page.size.width_mm, 1));
  });

  /** Photo déposée : dans une zone photo, elle la remplit ; ailleurs, une nouvelle photo de 80 mm de large. */
  function dropAsset(assetId: string, elementId: string | null, xMm: number, yMm: number) {
    const meta = editor.photos.find((p) => p.id === assetId) ?? editor.doc.assets[assetId];
    if (!meta) return;
    if (elementId) {
      editor.placePhoto(elementId, meta);
      editor.selection = [elementId];
      return;
    }
    const w = 80;
    const h = (w * meta.height_px) / meta.width_px;
    const id = newId();
    editor.commit({
      ...editor.doc,
      assets: { ...editor.doc.assets, [meta.id]: meta },
      pages: editor.doc.pages.map((p, i) =>
        i !== editor.pageIndex
          ? p
          : {
              ...p,
              elements: [
                ...p.elements,
                {
                  id,
                  frame: { x_mm: Math.round((xMm - w / 2) * 10) / 10, y_mm: Math.round((yMm - h / 2) * 10) / 10, w_mm: w, h_mm: Math.round(h * 10) / 10, rotation_deg: 0 },
                  locked: false,
                  zone: false,
                  kind: { type: "Photo", asset: meta.id, crop: { scale: 1, center_x: 0.5, center_y: 0.5 }, corner_radius_mm: 0, border: null, shadow: false },
                },
              ],
            },
      ),
    });
    editor.selection = [id];
  }

  function endCrop(id: string, crop: Crop) {
    editor.cropping = null;
    const e = editor.page.elements.find((x) => x.id === id);
    if (!e || e.kind.type !== "Photo") return;
    const c = e.kind.crop;
    if (c.scale === crop.scale && c.center_x === crop.center_x && c.center_y === crop.center_y) return;
    editor.updateElement(id, (el) => (el.kind.type === "Photo" ? { ...el, kind: { ...el.kind, crop } } : el));
  }

  function endTextEdit(id: string, content: RichText) {
    editor.editingText = null;
    const e = editor.page.elements.find((x) => x.id === id);
    if (!e || e.kind.type !== "Text") return;
    if (JSON.stringify(e.kind.content) === JSON.stringify(content)) return;
    editor.updateElement(id, (el) => (el.kind.type === "Text" ? { ...el, kind: { ...el.kind, content } } : el));
  }

  function select(id: string | null, add: boolean) {
    if (id === null) editor.selection = [];
    else if (add) editor.selection = editor.selection.includes(id) ? editor.selection.filter((s) => s !== id) : [...editor.selection, id];
    else if (editor.selection.length !== 1 || editor.selection[0] !== id) editor.selection = [id];
  }
</script>

<svelte:window onkeydown={handleShortcut} />

<div class="app">
  <Toolbar />
  {#if editor.templateMode}<TemplateBar />{/if}
  <LeftPanel />
  <main class="stage" bind:clientWidth={stageWidth}>
    <PageView
      page={editor.page}
      doc={editor.doc}
      zoom={editor.effectiveZoom}
      selection={editor.selection}
      cropping={editor.cropping}
      editingText={editor.editingText}
      templateMode={editor.templateMode}
      onselect={select}
      ondropasset={dropAsset}
      oncropstart={(id) => {
        editor.selection = [id];
        editor.cropping = id;
      }}
      oncropend={endCrop}
      onedittext={(id) => {
        editor.selection = [id];
        editor.editingText = id;
      }}
      ontextdone={endTextEdit}
      bind:pageEl
    />
    <Transformer {pageEl} zoom={editor.effectiveZoom} />
    {#if editor.showPicker}<TemplatePicker />{/if}
  </main>
  <RightPanel />
  <PageBar />
  <Modal />
  {#if editor.notice}
    <div class="notice" role="alert">
      <pre>{editor.notice}</pre>
      <button onclick={() => (editor.notice = null)}>OK</button>
    </div>
  {/if}
</div>
<PrintRoot />

<style>
  :global(:root) {
    --bg: #efe9df;
    --panel: #fbf8f3;
    --line: #e2d9cb;
    --text: #2d2a26;
    --muted: #8a8278;
    --accent: #2e86de;
    --accent-strong: #c0392b;
    --zone-photo: #2e86de;
    --zone-text: #e67e22;
    --ui-font: "Atkinson Hyperlegible", system-ui, sans-serif;
    --title-font: "Playwrite FR Moderne", cursive;
  }
  :global(html, body) {
    margin: 0;
    height: 100%;
    background: var(--bg);
    color: var(--text);
    font: 14px var(--ui-font);
  }
  :global(button, select, input) {
    font: inherit;
    color: inherit;
    border: 1px solid var(--line);
    background: white;
    border-radius: 6px;
    padding: 5px 10px;
  }
  :global(button:not(:disabled)) {
    cursor: pointer;
  }
  :global(button:not(:disabled):hover) {
    border-color: var(--accent);
  }
  :global(button:disabled) {
    opacity: 0.45;
  }
  :global(h3) {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
    margin: 4px 0 10px;
  }
  .app {
    display: grid;
    height: 100vh;
    grid-template-columns: 260px 1fr 260px;
    grid-template-rows: auto auto 1fr auto;
    grid-template-areas: "top top top" "tbar tbar tbar" "left stage right" "bottom bottom bottom";
  }
  .notice {
    position: fixed;
    left: 50%;
    bottom: 70px;
    transform: translateX(-50%);
    max-width: 640px;
    background: #fff4e5;
    border: 1px solid #f0b86e;
    border-radius: 10px;
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.15);
    padding: 12px 16px;
    display: flex;
    gap: 14px;
    align-items: flex-start;
    z-index: 100;
  }
  .notice pre {
    margin: 0;
    font: inherit;
    white-space: pre-wrap;
  }
  .stage {
    grid-area: stage;
    overflow: auto;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding: 32px;
    min-width: 0;
    position: relative;
  }
</style>
