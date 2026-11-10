<script lang="ts">
  // Pas à pas d'un guide (« Me guider ») : un anneau autour du vrai bouton de l'étape et une bulle qui dit quoi
  // faire. Rien n'est bloqué : on fait l'action dans son document, et l'étape suivante vient d'elle-même. On peut
  // aussi passer une étape ou arrêter.
  import Icon from "./Icon.svelte";
  import { tick } from "svelte";
  import { fr } from "../lib/i18n/fr";
  import { guideById, guides } from "../lib/guides.svelte";
  import { placeBubble, type Box } from "../lib/tour.svelte";

  const PAD = 6;
  const guide = $derived(guides.walking ? guideById(guides.walking.id) : null);
  const step = $derived(guide && guides.walking ? guide.walk[guides.walking.step] : null);
  let target = $state<Box | null>(null);
  let pos = $state<{ left: number; top: number } | null>(null);
  let bubbleEl = $state<HTMLElement>();

  async function measure() {
    const s = step;
    if (!s) return;
    const el = s.target ? document.querySelector<HTMLElement>(s.target) : null;
    // Un bouton dans une section repliée d'un tiroir : la section s'ouvre pour qu'on le voie.
    el?.closest("details:not([open])")?.setAttribute("open", "");
    const r = el?.getBoundingClientRect();
    target = r && r.width > 0 && r.height > 0 ? { left: r.left - PAD, top: r.top - PAD, width: r.width + 2 * PAD, height: r.height + 2 * PAD } : null;
    await tick();
    if (!bubbleEl) return;
    pos = placeBubble(target, { width: bubbleEl.offsetWidth, height: bubbleEl.offsetHeight }, { width: window.innerWidth, height: window.innerHeight });
  }

  // L'élément visé bouge avec le travail (tiroir ouvert, barre qui suit la sélection) : on le suit.
  $effect(() => {
    if (!step) return;
    void measure();
    const timer = setInterval(() => void measure(), 300);
    return () => clearInterval(timer);
  });

  // L'action de l'étape est faite : la suivante.
  $effect(() => {
    if (guides.walking) guides.advance();
  });
</script>

{#if guide && step && guides.walking}
  {#if target}
    <div class="ring" style:left="{target.left}px" style:top="{target.top}px" style:width="{target.width}px" style:height="{target.height}px"></div>
  {/if}
  <div class="bubble" bind:this={bubbleEl} role="status" aria-live="polite" style:left="{pos?.left ?? 0}px" style:top="{pos?.top ?? 0}px" style:visibility={pos ? "visible" : "hidden"}>
    <p class="kicker"><Icon name={guide.icon} size={14} /> {guide.title} · {guides.walking.step + 1}/{guide.walk.length}</p>
    <h3>{step.title}</h3>
    <p>{step.text}</p>
    <div class="buttons">
      <button class="link" onclick={() => guides.stop()}>{fr.guideStop}</button>
      <button onclick={() => guides.next()}>{guides.walking.step === guide.walk.length - 1 ? fr.guideDone : fr.guideSkipStep}</button>
    </div>
  </div>
{/if}

<style>
  .ring {
    position: fixed;
    border: 3px solid var(--accent);
    border-radius: 10px;
    box-shadow: 0 0 0 4px rgba(46, 134, 222, 0.25);
    pointer-events: none;
    z-index: 6500;
  }
  .bubble {
    position: fixed;
    width: 320px;
    background: white;
    border: 2px solid var(--accent);
    border-radius: 12px;
    padding: 12px 16px;
    box-shadow: 0 8px 28px rgba(0, 0, 0, 0.2);
    z-index: 6500;
  }
  .kicker {
    margin: 0 0 4px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--accent);
  }
  h3 {
    margin: 0 0 6px;
    font-size: 16px;
    text-transform: none;
    letter-spacing: 0;
    color: var(--text);
  }
  p {
    margin: 0;
    line-height: 1.4;
  }
  .buttons {
    display: flex;
    justify-content: space-between;
    margin-top: 10px;
  }
  .link {
    border: none;
    background: none;
    color: var(--muted);
    padding-left: 0;
  }
</style>
