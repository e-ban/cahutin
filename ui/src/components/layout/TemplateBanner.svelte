<script lang="ts">
  // Bandeau du mode modèle : une phrase d'aide, enregistrer comme modèle, terminer.
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
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
  <span class="text">▦ {fr.templateBanner}</span>
  <span class="spacer"></span>
  {#if naming}
    <!-- svelte-ignore a11y_autofocus -->
    <input placeholder={fr.templateName} bind:value={name} autofocus onkeydown={(e) => (e.key === "Enter" ? save() : e.key === "Escape" && (naming = false))} />
    <button class="primary" onclick={save} disabled={!name.trim()}>{fr.save}</button>
    <button onclick={() => (naming = false)}>{fr.cancel}</button>
  {:else}
    <button class="primary" onclick={() => (naming = true)}>💾 {fr.saveAsTemplate}</button>
  {/if}
  <button onclick={finish}>✓ {fr.finishTemplate}</button>
</div>

<style>
  .banner {
    grid-area: tbar;
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 6px 12px;
    background: #fff3dd;
    border-bottom: 1px solid #f0d9a8;
    color: #7a5200;
  }
  .spacer {
    flex: 1;
  }
  button {
    white-space: nowrap;
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
    font-weight: 600;
  }
</style>
