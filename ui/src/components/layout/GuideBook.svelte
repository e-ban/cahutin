<script lang="ts">
  // Fenêtre des guides : la liste (menu Aide > Guides), puis le livret d'un guide, un panneau à la fois, sa
  // capture animée et deux ou trois phrases. Au dernier panneau, « Me guider » lance le pas à pas sur l'écran.
  // Avec les animations réduites, la capture reste sur sa première image.
  import { fr } from "../../lib/i18n/fr";
  import { GUIDES, guideById, guides } from "../../lib/guides.svelte";
  import { settings } from "../../lib/settings.svelte";

  const guide = $derived(guides.open ? guideById(guides.open) : null);
  let i = $state(0);
  const panel = $derived(guide?.panels[i] ?? null);
  const last = $derived(!!guide && i === guide.panels.length - 1);

  function choose(id: string | null) {
    guides.open = id;
    i = 0;
  }

  // Animations réduites : la première image de la capture, dessinée une fois.
  function still(canvas: HTMLCanvasElement, src: string) {
    const draw = (s: string) => {
      const im = new Image();
      im.onload = () => {
        canvas.width = im.naturalWidth;
        canvas.height = im.naturalHeight;
        canvas.getContext("2d")?.drawImage(im, 0, 0);
      };
      im.src = s;
    };
    draw(src);
    return { update: draw };
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") guides.close();
    else if (!guide) return;
    else if (e.key === "ArrowRight" && !last) i++;
    else if (e.key === "ArrowLeft" && i > 0) i--;
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="backdrop" role="presentation" onclick={() => guides.close()}>
  <div class="box" role="dialog" aria-modal="true" aria-labelledby="guide-title" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
    {#if guide && panel}
      <div class="head">
        <button class="link" onclick={() => choose(null)}>← {fr.guidesAll}</button>
        <p class="kicker">{guide.icon} {guide.title} · {i + 1}/{guide.panels.length}</p>
      </div>
      <h2 id="guide-title">{panel.title}</h2>
      {#if settings.reduceMotion}
        <canvas class="shot" use:still={panel.image} aria-hidden="true"></canvas>
      {:else}
        <img class="shot" src={panel.image} alt="" />
      {/if}
      <p class="text">{panel.text}</p>
      <div class="foot">
        <div class="dots" aria-hidden="true">
          {#each guide.panels as _, j (j)}<button class="dot" class:on={j === i} tabindex="-1" onclick={() => (i = j)} aria-label={String(j + 1)}></button>{/each}
        </div>
        <span class="spacer"></span>
        {#if i > 0}<button onclick={() => i--}>{fr.whatsNewPrev}</button>{/if}
        {#if last}
          <button onclick={() => guides.close()}>{fr.close}</button>
          {#if guide.walk.length}<button class="primary" onclick={() => guides.walk(guide.id)}>👉 {fr.guideWalk}</button>{/if}
        {:else}
          <button class="primary" onclick={() => i++}>{fr.whatsNewNext}</button>
        {/if}
      </div>
    {:else}
      <div class="head">
        <h2 id="guide-title">📘 {fr.guides}</h2>
        <button onclick={() => guides.close()}>{fr.close}</button>
      </div>
      <p class="intro">{fr.guidesIntro}</p>
      <ul class="list">
        {#each GUIDES as g (g.id)}
          <li>
            <button class="item" onclick={() => choose(g.id)}>
              <span class="icon">{g.icon}</span>
              <span><b>{g.title}</b><small>{g.summary}</small></span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
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
  /* Hauteur fixe : la fenêtre ne saute pas d'un panneau à l'autre. */
  .shot {
    box-sizing: border-box;
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
  .intro {
    margin: 0;
    color: var(--muted);
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .item {
    width: 100%;
    display: flex;
    gap: 14px;
    align-items: center;
    text-align: left;
    padding: 12px 14px;
    border-radius: 10px;
  }
  .item .icon {
    font-size: 26px;
    width: 32px;
    text-align: center;
  }
  .item span:last-child {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .item small {
    color: var(--muted);
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
    padding-left: 0;
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
    font-weight: 600;
  }
</style>
