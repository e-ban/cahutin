<script lang="ts">
  import { modal } from "../lib/modal.svelte";
</script>

{#if modal.message}
  <div class="backdrop">
    <div class="box" role="dialog" aria-modal="true">
      <p>{modal.message}</p>
      <div class="buttons">
        {#each modal.choices as c}
          <button class:primary={c.primary} onclick={() => modal.answer(c.value)}>{c.label}</button>
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
    z-index: 200;
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
