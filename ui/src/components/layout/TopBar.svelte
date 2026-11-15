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
  import { firstFitting, topBarHas, TOP_BAR_MAX, type TopBarStep } from "../../lib/topBarFit";
  import { flushSync } from "svelte";

  let titleEl = $state<HTMLInputElement>();
  let barEl = $state<HTMLElement>();

  // Le palier de resserrement (lib/topBarFit.ts) : le premier où la dernière commande tient dans la barre. Tout est
  // essayé et mesuré avant que l'écran soit redessiné : on ne voit que le résultat.
  let level = $state(0);
  const has = (step: TopBarStep) => topBarHas(level, step);
  function fit() {
    const bar = barEl;
    if (!bar) return;
    const fits = (l: number) => {
      level = l;
      flushSync();
      const last = bar.lastElementChild;
      if (!last) return true;
      const end = bar.getBoundingClientRect().left + bar.clientWidth - parseFloat(getComputedStyle(bar).paddingRight);
      return last.getBoundingClientRect().right <= end + 0.5;
    };
    const l = firstFitting(fits);
    if (l !== level) {
      level = l;
      flushSync();
    }
  }
  $effect(() => {
    const bar = barEl;
    if (!bar) return;
    const ro = new ResizeObserver(() => fit());
    // La boîte entière : les marges que changent les paliers ne relancent pas la mesure.
    ro.observe(bar, { box: "border-box" });
    document.fonts?.ready.then(() => fit());
    return () => ro.disconnect();
  });
  // Ce qui change la largeur des commandes sans changer celle de la barre.
  $effect(() => {
    void [editor.templateMode, assistant.enabled, assistant.open, editor.dirty, settings.highContrast];
    queueMicrotask(fit);
  });

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

<!-- Une commande réduite garde son nom écrit pour les lecteurs d'écran (.sr) et sa bulle. -->
<header class="top" class:gaps={has("gaps")} class:tight={has("tight")} bind:this={barEl}>
  <button class="logo" class:short={has("appName")} title={fr.homeHint} onclick={goHome}><img src="/logo/cahutin.svg" alt="" /><span class:sr={has("appName")}>{fr.appName}</span></button>
  <FileMenu />
  <button class="ghost" title={fr.preferencesHint} onclick={() => (editor.showPreferences = true)}><Icon name="settings" /><span class:sr={has("toolIcons")}>{fr.preferences}</span></button>
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
    <Icon name="save" /><span class:sr={has("saveIcon")}>{fr.save}</span>
    {#if editor.dirty}<span class="dirty" aria-hidden="true"></span>{/if}
  </button>
  <!-- Le même réglage que Préférences > Enregistrement. Masquée en mode modèle, où rien ne s'enregistre tout seul.
       Un clic ne prend pas le focus : un texte en cours d'écriture reste ouvert (sur le libellé, WebKit donnerait le
       focus à la case : le clic y est fait ici). Au clavier (Tab, Espace), elle se coche comme une autre case. -->
  {#if !editor.templateMode}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions (le clavier va à la case, qui se coche d'elle-même) -->
    <label class="auto" title={editor.ownFile ? fr.autosaveToggleHint : fr.autosaveToggleHintNoFile} onmousedown={(e) => e.preventDefault()} onclick={labelClick}>
      <input type="checkbox" aria-label={fr.prefsAutosave} checked={settings.autosave} onchange={(e) => settings.set("autosave", e.currentTarget.checked)} /><span class:sr={has("autoBox")}>{fr.autosaveToggle}</span>
    </label>
  {/if}
  <div class="pair">
    <button onclick={() => editor.undo()} disabled={!editor.canUndo} title="{fr.undo} (Ctrl+Z)" aria-label={fr.undo}><Icon name="undo" /></button>
    <button onclick={() => editor.redo()} disabled={!editor.canRedo} title="{fr.redo} (Ctrl+Y)" aria-label={fr.redo}><Icon name="redo" /></button>
  </div>
  <span class="spacer"></span>
  <button class="ghost" class:tpl-on={editor.templateMode} data-guide="template-mode" title={has("toolIcons") ? fr.template : undefined} onclick={toggleTemplateMode}><Icon name="template" /><span class:sr={has("toolIcons")}>{fr.template}</span></button>
  {#if assistant.enabled}
    <button class="ghost" class:tpl-on={assistant.open} title={fr.assistant.buttonHint} onclick={() => (assistant.open = !assistant.open)}><Icon name="assistant" /><span class:sr={has("toolIcons")}>{fr.assistant.button}</span></button>
  {/if}
  <OutputMenu size={has("outputIcon") ? "icon" : has("shortOutput") ? "short" : "full"} />
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
  /* Les paliers de resserrement (lib/topBarFit.ts) : écarts réduits, puis au plus juste. */
  .top.gaps {
    gap: 6px;
    padding: 8px;
  }
  .top.gaps .logo {
    margin-right: 0;
  }
  .top.tight {
    gap: 4px;
    padding: 8px 6px;
  }
  .top.tight > button:not(.logo),
  .top.tight > :global(.menu > .trigger) {
    padding-left: 7px;
    padding-right: 7px;
  }
  .top.tight .title {
    min-width: 60px;
  }
  /* Le nom d'une commande réduite à son icône : plus écrit, toujours lu par les lecteurs d'écran. */
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
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
  /* Sans son nom, le logo garde la hauteur de la ligne écrite (un caractère sans largeur, l'écart annulé) : la barre
     ne change pas de hauteur et rien ne bouge en dessous. */
  .logo.short::before {
    content: "\200b";
    margin-right: -6px;
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
