<script lang="ts">
  import FileMenu from "./FileMenu.svelte";
  import HelpMenu from "./HelpMenu.svelte";
  import OutputMenu from "./OutputMenu.svelte";
  import { goHome, save } from "../../lib/fileActions";
  import { assistant } from "../../lib/assistant.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import Icon from "../Icon.svelte";

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
  <button class="ghost" title={fr.preferencesHint} onclick={() => (editor.showPreferences = true)}><Icon name="settings" />{fr.preferences}</button>
  <HelpMenu />
  <input
    class="title"
    value={editor.doc.title}
    placeholder={fr.untitled}
    aria-label="Titre du document"
    bind:this={titleEl}
    onchange={(e) => setTitle(e.currentTarget.value)}
  />
  <button class="save" title="{fr.saveHint} (Ctrl+S)" onclick={() => save()}>
    <Icon name="save" />{fr.save}
    {#if editor.dirty}<span class="dirty" role="img" title={fr.unsaved} aria-label={fr.unsaved}></span>{/if}
  </button>
  <div class="pair">
    <button onclick={() => editor.undo()} disabled={!editor.canUndo} title="{fr.undo} (Ctrl+Z)" aria-label={fr.undo}><Icon name="undo" /></button>
    <button onclick={() => editor.redo()} disabled={!editor.canRedo} title="{fr.redo} (Ctrl+Y)" aria-label={fr.redo}><Icon name="redo" /></button>
  </div>
  <span class="spacer"></span>
  <button class="ghost" class:tpl-on={editor.templateMode} data-guide="template-mode" onclick={toggleTemplateMode}><Icon name="template" />{fr.template}</button>
  {#if assistant.enabled}
    <button class="ghost" class:tpl-on={assistant.open} title={fr.assistant.buttonHint} onclick={() => (assistant.open = !assistant.open)}><Icon name="assistant" />{fr.assistant.button}</button>
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
  .top > button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }
  /* Bouton discret à bordure : « Imprimer / Exporter » reste le seul bouton plein. */
  .save {
    background: var(--panel);
    border-color: var(--line);
  }
  /* Le point des changements pas encore enregistrés, dans le bouton après le libellé. */
  .dirty {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
  }
  /* Annuler et Rétablir : une seule bordure autour des deux, un trait entre eux. */
  .pair {
    display: flex;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--panel);
  }
  .pair button {
    display: flex;
    border: 0;
    border-radius: 0;
    background: transparent;
    padding: 5px 8px;
  }
  .pair button:first-child {
    border-radius: 5px 0 0 5px;
  }
  .pair button:last-child {
    border-left: 1px solid var(--line);
    border-radius: 0 5px 5px 0;
  }
  .pair button:not(:disabled):hover {
    background: #eef5fc;
  }
  :global(:root[data-contrast]) .pair,
  :global(:root[data-contrast]) .pair button:last-child {
    border-width: 2px;
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
