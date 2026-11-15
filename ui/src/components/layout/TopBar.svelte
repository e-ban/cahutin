<script lang="ts">
  import FileMenu from "./FileMenu.svelte";
  import HelpMenu from "./HelpMenu.svelte";
  import OutputMenu from "./OutputMenu.svelte";
  import { goHome, save } from "../../lib/fileActions";
  import { assistant } from "../../lib/assistant.svelte";
  import { saveState } from "../../lib/autosave";
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

  // Le point du bouton « Enregistrer » : des changements ne sont pas dans le fichier (comme avant l'enregistrement
  // automatique : un document neuf jamais modifié n'en a pas). Enregistrement automatique en marche, il reste le
  // temps des quelques secondes d'attente. La bulle dit l'état en mots.
  const saveInfo = $derived(
    saveState({
      enabled: settings.autosave,
      ownFile: editor.ownFile,
      templateMode: editor.templateMode,
      dirty: editor.dirty,
      editing: !!editor.editingText,
      problem: autosave.problem,
      held: !!editor.classHold?.on,
    }),
  );
  const saveHints = {
    nothing: fr.statusNothing,
    auto: fr.statusAuto,
    editing: fr.statusEditingHint,
    editingManual: fr.statusEditingManualHint,
    pending: fr.statusPending,
    unsaved: fr.statusUnsaved,
    held: fr.statusClassHoldHint,
    template: fr.statusTemplateHint,
  };

  function labelClick(e: MouseEvent) {
    if (e.target instanceof HTMLInputElement) return;
    e.preventDefault();
    settings.set("autosave", !settings.autosave);
  }

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
  <button class="save" title={`${saveHints[saveInfo]}\n${fr.saveHint} (Ctrl+S)`} aria-label={editor.dirty ? fr.saveUnsavedLabel : undefined} onclick={() => save()}>
    <Icon name="save" />{fr.save}
    {#if editor.dirty}<span class="dirty" aria-hidden="true"></span>{/if}
  </button>
  <!-- Le même réglage que Préférences > Enregistrement. Masquée en mode modèle, où rien ne s'enregistre tout seul.
       Un clic ne prend pas le focus : un texte en cours d'écriture reste ouvert (sur le libellé, WebKit donnerait le
       focus à la case : le clic y est fait ici). Au clavier (Tab, Espace), elle se coche comme une autre case. -->
  {#if !editor.templateMode}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions (le clavier va à la case, qui se coche d'elle-même) -->
    <label class="auto" title={editor.ownFile ? fr.autosaveToggleHint : fr.autosaveToggleHintNoFile} onmousedown={(e) => e.preventDefault()} onclick={labelClick}>
      <input type="checkbox" aria-label={fr.prefsAutosave} checked={settings.autosave} onchange={(e) => settings.set("autosave", e.currentTarget.checked)} />{fr.autosaveToggle}
    </label>
  {/if}
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
  /* Le titre cède de la place quand la fenêtre est étroite : le reste de la barre reste entier. */
  .title {
    width: 240px;
    min-width: 80px;
    flex-shrink: 1;
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
  /* Le point des changements pas encore enregistrés, dans le bouton après le libellé. */
  .dirty {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
  }
  /* La case de l'enregistrement automatique, discrète, collée au bouton « Enregistrer ». */
  .auto {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-left: -4px;
    color: var(--muted);
    font-size: 0.92em;
    white-space: nowrap;
    cursor: pointer;
  }
  .auto input {
    margin: 0;
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
