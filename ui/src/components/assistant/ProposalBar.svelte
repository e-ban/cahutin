<script lang="ts">
  // Bandeau au-dessus de la page pendant une proposition de l'assistant, que le panneau soit ouvert ou non :
  // la page ne se modifie pas tant qu'on n'a pas accepté ou refusé.
  import Icon from "../Icon.svelte";
  import { assistant } from "../../lib/assistant.svelte";
  import { fr } from "../../lib/i18n/fr";

  const t = fr.assistant;
  const count = $derived(assistant.proposal?.steps.reduce((n, s) => n + Math.max(1, s.summary.length), 0) ?? 0);
</script>

<div class="banner" role="status">
  {#if assistant.viewing}
    <span class="text"><Icon name="assistant" /> {t.viewingStep}</span>
    <span class="spacer"></span>
    <button class="primary" onclick={() => assistant.view(null)}>{t.backToEnd}</button>
  {:else if assistant.proposal}
    <span class="text"><Icon name="assistant" /> {t.proposes(count)}. {t.locked}</span>
    <span class="spacer"></span>
    <button class="primary" onclick={() => assistant.accept()} disabled={assistant.busy}><Icon name="check" /> {t.accept}</button>
    <button onclick={() => assistant.refuse()} disabled={assistant.busy}><Icon name="x" /> {t.refuse}</button>
    {#if !assistant.open}<button onclick={() => (assistant.open = true)}>{t.showDetails}</button>{/if}
  {/if}
</div>

<style>
  .banner {
    grid-area: tbar;
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 8px 14px;
    background: var(--bg);
    border-bottom: 2px solid var(--accent);
  }
  .text {
    font-weight: 600;
  }
  .spacer {
    flex: 1;
  }
  .primary {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
</style>
