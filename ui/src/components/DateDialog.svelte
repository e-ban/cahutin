<script lang="ts">
  // La date d'une page (champs {date}, {annee_scolaire}…), demandée par dateAsk : le sélecteur de date de
  // l'ordinateur, ouvert sur la date du jour.
  import { dateAsk } from "../lib/dateAsk.svelte";
  import { fr } from "../lib/i18n/fr";
</script>

{#if dateAsk.open}
  <div class="backdrop">
    <div class="box" role="dialog" aria-modal="true" aria-labelledby="date-title">
      <h2 id="date-title">{fr.pageDateTitle}</h2>
      <p>{fr.pageDateHint}</p>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        type="date"
        bind:value={dateAsk.value}
        autofocus
        onkeydown={(e) => {
          if (e.key === "Enter") dateAsk.confirm();
          if (e.key === "Escape") dateAsk.close();
        }}
      />
      <div class="buttons">
        {#if dateAsk.cancellable}<button onclick={() => dateAsk.cancel()}>{fr.cancel}</button>{/if}
        <button class="primary" onclick={() => dateAsk.confirm()}>{fr.pageDateOk}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: var(--backdrop);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 6000;
  }
  .box {
    background: var(--panel);
    border-radius: 12px;
    padding: 20px 24px;
    max-width: 380px;
    box-shadow: 0 10px 40px var(--shadow);
  }
  h2 {
    margin: 0 0 6px;
    font-size: 17px;
  }
  p {
    margin: 0 0 14px;
    color: var(--muted);
  }
  input {
    font-size: 16px;
    padding: 6px 8px;
  }
  .buttons {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    margin-top: 18px;
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }
</style>
