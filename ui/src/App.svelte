<script lang="ts">
  import "./fonts.css";
  import LeftPanel from "./components/LeftPanel.svelte";
  import PageBar from "./components/PageBar.svelte";
  import PageView from "./components/PageView.svelte";
  import RightPanel from "./components/RightPanel.svelte";
  import Toolbar from "./components/Toolbar.svelte";
  import Transformer from "./components/Transformer.svelte";
  import { editor } from "./lib/docStore.svelte";
  import { sampleDocument } from "./lib/sample";
  import { handleShortcut } from "./lib/shortcuts";
  import { mmToPx } from "./lib/units";

  editor.reset(sampleDocument(), null);

  let stageWidth = $state(800);
  let pageEl = $state<HTMLElement>();
  $effect(() => {
    editor.fitZoom = Math.max(0.2, (stageWidth - 64) / mmToPx(editor.page.size.width_mm, 1));
  });

  function select(id: string | null, add: boolean) {
    if (id === null) editor.selection = [];
    else if (add) editor.selection = editor.selection.includes(id) ? editor.selection.filter((s) => s !== id) : [...editor.selection, id];
    else if (editor.selection.length !== 1 || editor.selection[0] !== id) editor.selection = [id];
  }
</script>

<svelte:window onkeydown={handleShortcut} />

<div class="app">
  <Toolbar />
  <LeftPanel />
  <main class="stage" bind:clientWidth={stageWidth}>
    <PageView page={editor.page} doc={editor.doc} zoom={editor.effectiveZoom} selection={editor.selection} onselect={select} bind:pageEl />
    <Transformer {pageEl} zoom={editor.effectiveZoom} />
  </main>
  <RightPanel />
  <PageBar />
</div>

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
    grid-template-rows: auto 1fr auto;
    grid-template-areas: "top top top" "left stage right" "bottom bottom bottom";
  }
  .stage {
    grid-area: stage;
    overflow: auto;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding: 32px;
    min-width: 0;
  }
</style>
