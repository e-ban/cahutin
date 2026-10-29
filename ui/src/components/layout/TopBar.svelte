<script lang="ts">
  import FileMenu from "./FileMenu.svelte";
  import HelpMenu from "./HelpMenu.svelte";
  import OutputMenu from "./OutputMenu.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";

  function toggleTemplateMode() {
    editor.templateMode = !editor.templateMode;
    editor.drawer = editor.templateMode ? "grid" : null;
    editor.selection = [];
  }
</script>

<header class="top">
  <span class="logo">{fr.appName}</span>
  <FileMenu />
  <HelpMenu />
  <input
    class="title"
    value={editor.doc.title}
    placeholder={fr.untitled}
    aria-label="Titre du document"
    onchange={(e) => editor.commit({ ...editor.doc, title: e.currentTarget.value })}
  />
  {#if editor.dirty}<span class="dirty" title={fr.unsaved}>•</span>{/if}
  <button class="ghost" onclick={() => editor.undo()} disabled={!editor.canUndo} title="{fr.undo} (Ctrl+Z)" aria-label={fr.undo}>↶</button>
  <button class="ghost" onclick={() => editor.redo()} disabled={!editor.canRedo} title="{fr.redo} (Ctrl+Y)" aria-label={fr.redo}>↷</button>
  <span class="spacer"></span>
  <button class="ghost" class:tpl-on={editor.templateMode} onclick={toggleTemplateMode}>▦ {fr.template}</button>
  <OutputMenu />
</header>

<style>
  .top {
    grid-area: top;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    background: var(--panel);
    border-bottom: 1px solid var(--line);
  }
  .logo {
    font: 700 18px var(--title-font);
    color: var(--accent-strong);
    margin-right: 4px;
  }
  .title {
    width: 240px;
    border-color: transparent;
    background: transparent;
  }
  .title:hover,
  .title:focus {
    border-color: var(--line);
    background: white;
  }
  .dirty {
    color: var(--accent);
    font-size: 20px;
    line-height: 1;
  }
  .ghost {
    border-color: transparent;
    background: transparent;
  }
  .tpl-on {
    background: #fff3dd;
    border-color: #e6a23c;
    color: #9a6300;
    font-weight: 600;
  }
  .spacer {
    flex: 1;
  }
</style>
