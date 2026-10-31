<script lang="ts">
  // Fenêtre des nouveautés : une nouveauté à la fois, sa capture d'écran et deux phrases. Elle s'ouvre sur la
  // dernière version ; le choix de la version, en haut, retrouve celles d'avant.
  import { fr } from "../../lib/i18n/fr";
  import { RELEASES } from "../../lib/whatsNew";

  let { onclose }: { onclose: () => void } = $props();

  let release = $state(RELEASES[0]);
  let i = $state(0);
  const item = $derived(release.items[i]);
  const last = $derived(i === release.items.length - 1);

  function pick(version: string) {
    release = RELEASES.find((r) => r.version === version) ?? RELEASES[0];
    i = 0;
  }
</script>

<svelte:window onkeydown={(e) => (e.key === "Escape" ? onclose() : e.key === "ArrowRight" ? !last && i++ : e.key === "ArrowLeft" && i > 0 && i--)} />

<div class="backdrop">
  <div class="box" role="dialog" aria-modal="true" aria-labelledby="news-title">
    <div class="head">
      <p class="kicker">{fr.whatsNewKicker(release.version)}</p>
      {#if RELEASES.length > 1}
        <select aria-label={fr.whatsNewVersions} value={release.version} onchange={(e) => pick(e.currentTarget.value)}>
          {#each RELEASES as r (r.version)}<option value={r.version}>{fr.whatsNewVersion(r.version)}</option>{/each}
        </select>
      {/if}
    </div>
    <h2 id="news-title">{item.title}</h2>
    <img src={item.image} alt="" />
    <p class="text">{item.text}</p>
    <div class="foot">
      <div class="dots" aria-hidden="true">
        {#each release.items as _, j (j)}<button class="dot" class:on={j === i} tabindex="-1" onclick={() => (i = j)} aria-label={String(j + 1)}></button>{/each}
      </div>
      <span class="spacer"></span>
      {#if i > 0}<button onclick={() => i--}>{fr.whatsNewPrev}</button>{/if}
      {#if last}
        <button class="primary" onclick={onclose}>{fr.whatsNewDone}</button>
      {:else}
        <button class="link" onclick={onclose}>{fr.whatsNewSkip}</button>
        <button class="primary" onclick={() => i++}>{fr.whatsNewNext}</button>
      {/if}
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(40, 30, 20, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 7000;
  }
  .box {
    background: var(--panel);
    border-radius: 14px;
    padding: 20px 24px;
    width: min(92vw, 820px);
    max-height: 94vh;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.25);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .head select {
    font-size: 12px;
  }
  .kicker {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--accent);
  }
  h2 {
    margin: 0;
    font-size: 22px;
  }
  /* Hauteur fixe : la fenêtre ne saute pas d'une nouveauté à l'autre. */
  img {
    width: 100%;
    height: min(52vh, 480px);
    object-fit: contain;
    border-radius: 8px;
    border: 1px solid var(--line);
    background: #f3efe8;
  }
  .text {
    margin: 0;
    font-size: 15px;
    line-height: 1.45;
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .spacer {
    flex: 1;
  }
  .dots {
    display: flex;
    gap: 6px;
  }
  .dot {
    width: 9px;
    height: 9px;
    padding: 0;
    border-radius: 50%;
    border: none;
    background: var(--line);
  }
  .dot.on {
    background: var(--accent);
  }
  .link {
    border: none;
    background: none;
    color: var(--muted);
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
    font-weight: 600;
  }
</style>
