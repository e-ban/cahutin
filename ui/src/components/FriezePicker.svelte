<script lang="ts">
  // Choix des motifs d'une frise, dans une fenêtre : un clic ajoute l'image à la suite (numérotée), un second
  // l'en retire.
  import PictoPanel from "./drawers/PictoPanel.svelte";
  import { toggleMotif } from "../lib/decor";
  import { friezeMotifs } from "../lib/friezeMotifs.svelte";
  import { fr } from "../lib/i18n/fr";
  import { pictoUrl } from "../lib/pictos.svelte";

  let { onclose }: { onclose: () => void } = $props();
  let query = $state("");
</script>

<div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()} onkeydown={(e) => e.key === "Escape" && onclose()}>
  <div class="box" role="dialog" aria-modal="true" aria-labelledby="picker-title">
    <h2 id="picker-title">{fr.friezePickTitle}</h2>
    <p class="hint">{fr.friezePickHint}</p>
    <div class="chosen">
      {#each friezeMotifs.codes as code, i (code)}
        <span class="motif"><img src={pictoUrl(code)} alt="" /><b>{i + 1}</b></span>
      {:else}
        <span class="hint">{fr.friezeNoMotif}</span>
      {/each}
    </div>
    <input class="search" type="search" bind:value={query} placeholder={fr.searchPicture} aria-label={fr.searchPicture} />
    <div class="scroll">
      <PictoPanel {query} chosen={friezeMotifs.codes} onpick={(code) => (friezeMotifs.codes = toggleMotif(friezeMotifs.codes, code))} />
    </div>
    <div class="buttons">
      <button class="primary" onclick={onclose}>✓ {fr.friezePickDone}</button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(40, 30, 20, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 6000;
  }
  .box {
    background: var(--panel);
    border-radius: 12px;
    padding: 20px 24px;
    width: min(720px, 92vw);
    height: min(640px, 88vh);
    display: flex;
    flex-direction: column;
    gap: 10px;
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.25);
  }
  h2 {
    margin: 0;
    font-size: 18px;
  }
  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }
  .chosen {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    min-height: 36px;
    align-items: center;
  }
  .motif {
    position: relative;
  }
  .motif img {
    width: 32px;
    height: 32px;
    display: block;
  }
  .motif b {
    position: absolute;
    top: -4px;
    right: -4px;
    min-width: 14px;
    height: 14px;
    border-radius: 7px;
    background: var(--accent);
    color: white;
    font-size: 10px;
    line-height: 14px;
    text-align: center;
  }
  .search {
    padding: 6px 8px;
  }
  .scroll {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
  }
  .primary {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
    padding: 8px 16px;
  }
</style>
