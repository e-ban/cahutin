<script lang="ts">
  import { modal } from "../lib/modal.svelte";

  // Le choix proposé a le focus à l'ouverture : Entrée le prend, Échap revient en arrière.
  function focusIf(node: HTMLButtonElement, on: boolean | undefined) {
    if (on) node.focus();
  }

  // En capture, avant les autres raccourcis : la touche ferme la boîte et rien d'autre (accueil, page Classe).
  function onkeydown(e: KeyboardEvent) {
    if (!modal.message || e.key !== "Escape") return;
    e.preventDefault();
    e.stopPropagation();
    modal.cancel();
  }
</script>

<svelte:window onkeydowncapture={onkeydown} />

{#if modal.message}
  <div class="backdrop">
    <div class="box" role="dialog" aria-modal="true">
      <p>{modal.message}</p>
      <div class="buttons">
        {#each modal.choices as c}
          <button class:primary={c.primary} use:focusIf={c.primary} onclick={() => modal.answer(c.value)}>{c.label}</button>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(40, 30, 20, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 6000; /* au-dessus de l'écran d'accueil */
  }
  .box {
    background: var(--panel);
    border-radius: 12px;
    padding: 20px 24px;
    max-width: 440px;
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.25);
  }
  p {
    margin: 0 0 18px;
    font-size: 15px;
  }
  .buttons {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
  .primary {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
</style>
