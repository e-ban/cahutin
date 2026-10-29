<script lang="ts">
  // Calque de la visite guidée : la zone de l'étape est mise en lumière, le reste assombri, et une bulle
  // explique. Échap passe la visite, les flèches naviguent ; aucune autre touche n'atteint l'application.
  import { tick } from "svelte";
  import { fr } from "../lib/i18n/fr";
  import { placeBubble, tour, TOUR_STEPS, type Box } from "../lib/tour.svelte";

  const PAD = 6;
  let target = $state<Box | null>(null);
  let pos = $state<{ left: number; top: number } | null>(null);
  let bubbleEl = $state<HTMLElement>();

  const step = $derived(tour.step === null ? null : TOUR_STEPS[tour.step]);

  async function measure() {
    const s = step;
    if (!s) return;
    const el = s.target ? document.querySelector<HTMLElement>(s.target) : null;
    const r = el?.getBoundingClientRect();
    target = r && r.width > 0 && r.height > 0 ? { left: r.left - PAD, top: r.top - PAD, width: r.width + 2 * PAD, height: r.height + 2 * PAD } : null;
    await tick();
    if (!bubbleEl) return;
    pos = placeBubble(target, { width: bubbleEl.offsetWidth, height: bubbleEl.offsetHeight }, { width: window.innerWidth, height: window.innerHeight });
  }

  $effect(() => {
    void tour.step;
    void measure();
  });

  $effect(() => {
    if (tour.step === null) return;
    const onKey = (e: KeyboardEvent) => {
      e.stopPropagation();
      if (e.key === "Escape") tour.end();
      else if (e.key === "ArrowRight") tour.next();
      else if (e.key === "ArrowLeft") tour.prev();
      else if (e.key !== "Tab" && e.key !== "Enter" && e.key !== " ") e.preventDefault();
    };
    const onResize = () => void measure();
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("resize", onResize);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("resize", onResize);
    };
  });
</script>

{#if step !== null && tour.step !== null}
  <div class="tour" class:dim={!target}>
    {#if target}
      <div class="spot" style:left="{target.left}px" style:top="{target.top}px" style:width="{target.width}px" style:height="{target.height}px"></div>
    {/if}
    <div
      class="bubble"
      role="dialog"
      aria-modal="true"
      aria-labelledby="tour-title"
      bind:this={bubbleEl}
      style:left={pos ? `${pos.left}px` : "-9999px"}
      style:top={pos ? `${pos.top}px` : "-9999px"}
    >
      <h2 id="tour-title">{step.title}</h2>
      <p>{step.text}</p>
      <div class="foot">
        <span class="count">{fr.tourStep(tour.step + 1, TOUR_STEPS.length)}</span>
        <button class="link" onclick={() => tour.end()}>{fr.tourSkip}</button>
        <button onclick={() => tour.prev()} disabled={tour.step === 0}>{fr.tourPrev}</button>
        <button class="primary" onclick={() => tour.next()}>{tour.step === TOUR_STEPS.length - 1 ? fr.tourDone : fr.tourNext}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .tour {
    position: fixed;
    inset: 0;
    z-index: 5500; /* au-dessus de la barre flottante et des menus, sous les boîtes de dialogue (6000) */
  }
  .tour.dim {
    background: rgba(0, 0, 0, 0.55);
  }
  .spot {
    position: fixed;
    border-radius: 10px;
    box-shadow: 0 0 0 9999px rgba(0, 0, 0, 0.55);
    outline: 3px solid var(--accent);
    pointer-events: none;
    transition: all 0.2s ease;
  }
  .bubble {
    position: fixed;
    width: 320px;
    background: var(--panel);
    color: var(--text);
    border-radius: 12px;
    padding: 16px 18px 12px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.3);
    font-family: var(--ui-font);
  }
  h2 {
    margin: 0 0 6px;
    font-size: 17px;
  }
  p {
    margin: 0 0 14px;
    line-height: 1.45;
    font-size: 14px;
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .count {
    flex: 1;
    color: var(--muted);
    font-size: 12px;
  }
  button {
    font: inherit;
    font-size: 13px;
    padding: 6px 10px;
    border-radius: 8px;
    border: 1px solid var(--line);
    background: #fff;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  button.link {
    border: none;
    background: none;
    color: var(--muted);
    text-decoration: underline;
  }
</style>
