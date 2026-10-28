<script lang="ts">
  // Fond de la page : couleur, papier kraft, motifs d'école ; appliquer à toutes les pages.
  import ColorPicker from "./ColorPicker.svelte";
  import { KRAFT, marks } from "../lib/background";
  import { editor } from "../lib/docStore.svelte";
  import { setBackground } from "../lib/editActions";
  import { fr } from "../lib/i18n/fr";
  import type { Pattern } from "../lib/types";

  const patterns: { pattern: Pattern | null; label: string }[] = [
    { pattern: null, label: fr.patternNone },
    { pattern: "Seyes", label: fr.patternSeyes },
    { pattern: "SmallSquares", label: fr.patternSquares },
    { pattern: "Lines", label: fr.patternLines },
    { pattern: "Dots", label: fr.patternDots },
  ];
  const PREVIEW = { width_mm: 30, height_mm: 40 };

  const current = $derived(editor.page.background ?? { color: "#ffffff", pattern: null });

  function apply(color: string, pattern: Pattern | null) {
    setBackground(color === "#ffffff" && !pattern ? null : { color, pattern }, false);
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && (editor.showBackground = false)} />

<div class="panel" role="dialog" aria-label={fr.background}>
  <header>
    <h3>{fr.background}</h3>
    <button class="close" aria-label={fr.close} onclick={() => (editor.showBackground = false)}>×</button>
  </header>
  <span class="label">{fr.backgroundColor}</span>
  <ColorPicker value={current.color} onchange={(c) => apply(c ?? "#ffffff", current.pattern ?? null)} />
  <button class="kraft" class:on={current.color === KRAFT} onclick={() => apply(KRAFT, current.pattern ?? null)}><i style:background={KRAFT}></i>{fr.kraft}</button>
  <span class="label">{fr.backgroundPattern}</span>
  <div class="patterns">
    {#each patterns as p (p.label)}
      <button class:on={(current.pattern ?? null) === p.pattern} onclick={() => apply(current.color, p.pattern)}>
        <svg viewBox="0 0 30 40" style:background={current.color}>
          {#if p.pattern}
            {#each marks(PREVIEW, p.pattern) as m, i (i)}
              {#if m.type === "Line"}<line x1={m.x1} y1={m.y1} x2={m.x2} y2={m.y2} stroke={m.color} stroke-width={Math.max(0.6, m.width_mm * 4)} />{:else}<circle cx={m.x} cy={m.y} r={m.d_mm * 1.5} fill={m.color} />{/if}
            {/each}
          {/if}
        </svg>
        {p.label}
      </button>
    {/each}
  </div>
  <button class="all" onclick={() => setBackground(editor.page.background ?? null, true)}>{fr.applyToAllPages}</button>
</div>

<style>
  .panel {
    position: fixed;
    right: 24px;
    bottom: 110px;
    width: 300px;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 10px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
    padding: 10px 12px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 4000;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  h3 {
    margin: 0;
  }
  .close {
    border: 0;
    background: transparent;
    font-size: 18px;
  }
  .label {
    font-size: 12px;
    color: var(--muted);
  }
  .kraft {
    display: flex;
    align-items: center;
    gap: 6px;
    align-self: flex-start;
  }
  .kraft i {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    display: inline-block;
  }
  .patterns {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 4px;
  }
  .patterns button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    padding: 4px 2px;
    font-size: 10px;
  }
  .patterns svg {
    width: 30px;
    height: 40px;
    border: 1px solid var(--line);
  }
  .on {
    border-color: var(--accent);
    background: #e8f1fb;
  }
</style>
