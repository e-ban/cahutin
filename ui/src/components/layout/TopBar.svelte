<script lang="ts">
  import FileMenu from "./FileMenu.svelte";
  import HelpMenu from "./HelpMenu.svelte";
  import OutputMenu from "./OutputMenu.svelte";
  import { goHome, save } from "../../lib/fileActions";
  import { assistant } from "../../lib/assistant.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";

  let titleEl = $state<HTMLInputElement>();

  function setTitle(title: string) {
    if (title !== editor.doc.title) editor.commit({ ...editor.doc, title });
  }

  // Enregistrer (Ctrl+S) pendant la frappe du titre : le titre tapé compte, il devient le nom proposé.
  $effect(() =>
    editor.onFlush(() => {
      if (titleEl && document.activeElement === titleEl) setTitle(titleEl.value);
    }),
  );

  function toggleTemplateMode() {
    editor.templateMode = !editor.templateMode;
    editor.drawer = editor.templateMode ? "grid" : null;
    editor.selection = [];
  }
</script>

<header class="top">
  <button class="logo" title={fr.homeHint} onclick={goHome}><img src="/logo/cahutin.svg" alt="" />{fr.appName}</button>
  <FileMenu />
  <button class="ghost" title={fr.preferencesHint} onclick={() => (editor.showPreferences = true)}>⚙ {fr.preferences}</button>
  <HelpMenu />
  <input
    class="title"
    value={editor.doc.title}
    placeholder={fr.untitled}
    aria-label="Titre du document"
    bind:this={titleEl}
    onchange={(e) => setTitle(e.currentTarget.value)}
  />
  {#if editor.dirty}<span class="dirty" title={fr.unsaved}>•</span>{/if}
  <button class="save" title="{fr.saveHint} (Ctrl+S)" onclick={() => save()}>💾 {fr.save}</button>
  <button class="ghost" onclick={() => editor.undo()} disabled={!editor.canUndo} title="{fr.undo} (Ctrl+Z)" aria-label={fr.undo}>↶</button>
  <button class="ghost" onclick={() => editor.redo()} disabled={!editor.canRedo} title="{fr.redo} (Ctrl+Y)" aria-label={fr.redo}>↷</button>
  <span class="spacer"></span>
  <button class="ghost" class:tpl-on={editor.templateMode} data-guide="template-mode" onclick={toggleTemplateMode}>▦ {fr.template}</button>
  {#if assistant.enabled}
    <button class="ghost" class:tpl-on={assistant.open} title={fr.assistant.buttonHint} onclick={() => (assistant.open = !assistant.open)}>✦ {fr.assistant.button}</button>
  {/if}
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
    border: none;
    background: none;
    padding: 2px 6px;
    border-radius: 8px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 6px;
    font: 700 18px var(--title-font);
    color: var(--accent-strong);
    margin-right: 4px;
  }
  .logo:hover {
    background: var(--bg);
  }
  .logo img {
    width: 30px;
    height: 30px;
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
  /* Comme « Imprimer / Exporter » : la commande qu'on cherche, bien visible. */
  .save {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
    font-weight: 600;
    white-space: nowrap;
  }
  .save:hover {
    filter: brightness(1.08);
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
