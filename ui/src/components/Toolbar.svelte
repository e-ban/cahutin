<script lang="ts">
  import { editor, type Zoom } from "../lib/docStore.svelte";
  import { fr } from "../lib/i18n/fr";

  const levels = [0.5, 0.75, 1, 1.25, 1.5, 2];
</script>

<header class="toolbar">
  <div class="brand">{fr.appName}</div>
  <input
    class="title"
    value={editor.doc.title}
    placeholder={fr.untitled}
    onchange={(e) => editor.commit({ ...editor.doc, title: e.currentTarget.value })}
  />
  <div class="group">
    <button onclick={() => editor.undo()} disabled={!editor.canUndo} title="Ctrl+Z">↶ {fr.undo}</button>
    <button onclick={() => editor.redo()} disabled={!editor.canRedo} title="Ctrl+Y">↷ {fr.redo}</button>
  </div>
  <label class="group">
    {fr.zoom}
    <select
      value={String(editor.zoom)}
      onchange={(e) => {
        const v = e.currentTarget.value;
        editor.zoom = (v === "fit" ? "fit" : Number(v)) as Zoom;
      }}
    >
      <option value="fit">{fr.zoomFit}</option>
      {#each levels as l}<option value={String(l)}>{Math.round(l * 100)} %</option>{/each}
    </select>
  </label>
  <button class:on={editor.templateMode} onclick={() => (editor.templateMode = !editor.templateMode)}>
    ▦ {editor.templateMode ? "Terminer le modèle" : "Modifier le modèle"}
  </button>
  <div class="spacer"></div>
</header>

<style>
  .toolbar {
    grid-area: top;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 14px;
    background: var(--panel);
    border-bottom: 1px solid var(--line);
  }
  .brand {
    font: 700 18px var(--title-font);
    color: var(--accent-strong);
  }
  .title {
    width: 260px;
  }
  .group {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .on {
    background: #fff7e8;
    border-color: #e6a23c;
  }
  .spacer {
    flex: 1;
  }
</style>
