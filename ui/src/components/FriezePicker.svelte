<script lang="ts">
  // Choix des motifs d'une frise, dans une fenêtre : un clic ajoute l'image à la suite (numérotée), un second
  // l'en retire.
  import MyImagesPanel from "./drawers/MyImagesPanel.svelte";
  import PictoPanel from "./drawers/PictoPanel.svelte";
  import { toggleMotif } from "../lib/decor";
  import { motifUrl } from "../lib/friezeMotifs.svelte";
  import { fr } from "../lib/i18n/fr";

  // `codes` : la suite choisie, modifiée sur place (motifs du tiroir Décors, ou d'une frise posée).
  let { codes = $bindable(), onclose }: { codes: string[]; onclose: () => void } = $props();
  let query = $state("");
  let tab = $state<"pictos" | "mine">("pictos");
  const pick = (key: string) => (codes = toggleMotif(codes, key));
</script>

<div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()} onkeydown={(e) => e.key === "Escape" && onclose()}>
  <div class="box" role="dialog" aria-modal="true" aria-labelledby="picker-title">
    <h2 id="picker-title">{fr.friezePickTitle}</h2>
    <p class="hint">{fr.friezePickHint}</p>
    <div class="chosen">
      {#each codes as code, i (code)}
        <!-- Un clic sur un motif choisi le retire (aussi une image déjà dans la frise, absente des listes). -->
        <button class="motif" title={fr.removeMotif} aria-label={`${fr.removeMotif} ${i + 1}`} onclick={() => (codes = codes.filter((_, j) => j !== i))}><img src={motifUrl(code)} alt="" /><b>{i + 1}</b></button>
      {:else}
        <span class="hint">{fr.friezeNoMotif}</span>
      {/each}
    </div>
    <div class="tabs" role="tablist" aria-label={fr.friezePickTabs}>
      <button role="tab" aria-selected={tab === "pictos"} class:on={tab === "pictos"} onclick={() => (tab = "pictos")}>{fr.pictograms}</button>
      <button role="tab" aria-selected={tab === "mine"} class:on={tab === "mine"} onclick={() => (tab = "mine")}>{fr.myImages}</button>
    </div>
    <input class="search" type="search" bind:value={query} placeholder={fr.searchPicture} aria-label={fr.searchPicture} />
    <div class="scroll">
      {#if tab === "pictos"}
        <PictoPanel {query} chosen={codes} onpick={pick} />
      {:else}
        <MyImagesPanel {query} chosen={codes} onpick={pick} />
      {/if}
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
    padding: 2px;
    border: 1px solid transparent;
    background: transparent;
  }
  .motif:hover {
    border-color: #c0392b;
    background: #fdecea;
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
  .tabs {
    display: flex;
    align-self: flex-start;
    border: 1px solid var(--line);
    border-radius: 8px;
    overflow: hidden;
  }
  .tabs button {
    border: 0;
    border-radius: 0;
    padding: 6px 14px;
    background: white;
  }
  .tabs button + button {
    border-left: 1px solid var(--line);
  }
  .tabs .on {
    background: #e8f1fb;
    color: var(--accent);
    font-weight: 600;
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
