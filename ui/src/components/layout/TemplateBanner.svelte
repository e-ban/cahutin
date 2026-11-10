<script lang="ts">
  // Bandeau du mode modèle : une phrase d'aide, enregistrer comme modèle, terminer, le guide.
  import Icon from "../Icon.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { guides } from "../../lib/guides.svelte";
  import { templates } from "../../lib/templates.svelte";

  let naming = $state(false);
  let name = $state("");

  async function save() {
    const n = name.trim();
    if (!n) return;
    try {
      await templates.saveCurrentPage(n);
      editor.notice = fr.templateSaved(n);
      naming = false;
      name = "";
    } catch (e) {
      editor.notice = String(e);
    }
  }

  function finish() {
    editor.templateMode = false;
    editor.drawer = null;
    editor.selection = [];
  }
</script>

<div class="banner">
  <span class="text"><Icon name="template" /> {fr.templateBanner}</span>
  <span class="spacer"></span>
  {#if naming}
    <!-- svelte-ignore a11y_autofocus -->
    <input placeholder={fr.templateName} bind:value={name} autofocus onkeydown={(e) => (e.key === "Enter" ? save() : e.key === "Escape" && (naming = false))} />
    <button class="primary" onclick={save} disabled={!name.trim()}>{fr.save}</button>
    <button onclick={() => (naming = false)}>{fr.cancel}</button>
  {:else}
    <button class="primary" data-guide="save-template" onclick={() => (naming = true)}><Icon name="save" /> {fr.saveAsTemplate}</button>
  {/if}
  <button data-guide="finish-template" onclick={finish}><Icon name="check" /> {fr.finishTemplate}</button>
  <button class="help" title={fr.guideHelp} onclick={() => guides.show("modele")}><Icon name="help" /> {fr.guideHelp}</button>
</div>

<style>
  .banner {
    grid-area: tbar;
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 6px 12px;
    background: var(--warn-bg);
    border-bottom: 1px solid var(--warn-line);
    color: var(--text);
  }
  .spacer {
    flex: 1;
  }
  button {
    white-space: nowrap;
  }
  .help {
    border-color: transparent;
    background: transparent;
    color: inherit;
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }
</style>
