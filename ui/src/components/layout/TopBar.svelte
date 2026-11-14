<script lang="ts">
  import FileMenu from "./FileMenu.svelte";
  import HelpMenu from "./HelpMenu.svelte";
  import OutputMenu from "./OutputMenu.svelte";
  import { goHome, save } from "../../lib/fileActions";
  import { assistant } from "../../lib/assistant.svelte";
  import { saveStatus } from "../../lib/autosave";
  import { autosave } from "../../lib/autosave.svelte";
  import { settings } from "../../lib/settings.svelte";
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

  // « Enregistré », « Enregistrement… » ou « Non enregistré », toujours visible à côté du titre.
  const editing = $derived(!!editor.editingText || !!editor.cropping);
  // En mode modèle, rien ne s'enregistre tout seul (lib/autosave.svelte.ts) : « Non enregistré » s'il y a des changements.
  const ownFile = $derived(editor.ownFile && !editor.templateMode);
  // Après un remplissage par la classe, rien ne s'enregistre tout seul avant un enregistrement à la main.
  const held = $derived(!!editor.classHold?.on);
  const status = $derived(saveStatus({ enabled: settings.autosave, path: editor.path, ownFile, dirty: editor.dirty, editing, problem: autosave.problem, held }));
  const statusLabel = $derived(status === "saved" ? fr.statusSaved : status === "saving" ? fr.statusSaving : fr.statusUnsaved);
  const statusHint = $derived(
    settings.autosave && editor.ownFile && held && status === "unsaved"
      ? fr.statusClassHoldHint
      : settings.autosave && editor.ownFile && editor.templateMode && status !== "saving" && !autosave.problem
      ? fr.statusTemplateHint
      : status === "saving"
      ? editing
        ? fr.statusEditingHint
        : fr.statusSavingHint
      : status === "saved"
        ? settings.autosave && ownFile
          ? fr.statusSavedHint
          : fr.statusSavedManualHint
        : autosave.problem?.kind === "failed"
          ? autosave.problem.message
          : settings.autosave
            ? fr.statusUnsavedHint
            : fr.statusUnsavedManualHint,
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
  <!-- Non enregistré : un clic enregistre (ou demande où, la première fois). -->
  <button class="status {status}" title={statusHint} aria-live="polite" aria-disabled={status !== "unsaved"} onclick={() => status === "unsaved" && save()}>
    <Icon name={status === "saved" ? "check" : status === "saving" ? "history" : "alert"} />{statusLabel}
  </button>
  <button class="save" title="{fr.saveHint} (Ctrl+S)" onclick={() => save()}>
    <Icon name="save" />{fr.save}
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
    background: var(--surface);
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
  /* L'état de l'enregistrement : un texte calme, sans animation ; « Non enregistré » se remarque et se clique. */
  .status {
    border-color: transparent;
    background: transparent;
    color: var(--muted);
    font-size: 0.92em;
    padding: 4px 6px;
  }
  .status[aria-disabled="true"] {
    cursor: default;
  }
  .status.unsaved {
    color: var(--text);
    background: var(--warn-bg);
    border-color: var(--warn-line);
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
    background: var(--accent-soft);
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
    background: var(--warn-bg);
    border-color: var(--warn-line);
    color: var(--text);
    font-weight: 600;
  }
  .spacer {
    flex: 1;
  }
</style>
