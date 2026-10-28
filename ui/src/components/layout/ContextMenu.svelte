<script lang="ts" module>
  export interface ContextTarget {
    /** Élément visé, ou null pour la page vide. */
    id: string | null;
    xMm: number;
    yMm: number;
    clientX: number;
    clientY: number;
  }
</script>

<script lang="ts">
  // Menu du clic droit : sur un élément, ou sur la page vide.
  import { editor } from "../../lib/docStore.svelte";
  import { bringSelection, copySelection, cutSelection, deleteSelection, duplicateSelection, hasClipboard, paste, toggleLockSelection } from "../../lib/editActions";
  import { newId, textElement } from "../../lib/factory";
  import { fr } from "../../lib/i18n/fr";

  let { target, onclose }: { target: ContextTarget; onclose: () => void } = $props();

  let menuEl = $state<HTMLElement>();
  const element = $derived(target.id ? editor.page.elements.find((e) => e.id === target.id) : undefined);

  // Reste dans la fenêtre même près des bords.
  const left = $derived(Math.min(target.clientX, window.innerWidth - (menuEl?.offsetWidth ?? 220) - 8));
  const top = $derived(Math.min(target.clientY, window.innerHeight - (menuEl?.offsetHeight ?? 300) - 8));

  function run(action: () => void) {
    onclose();
    action();
  }

  function addHere(kind: "text" | "shape") {
    const w = kind === "text" ? 80 : 60;
    const h = kind === "text" ? 20 : 40;
    const frame = { x_mm: Math.round(target.xMm - w / 2), y_mm: Math.round(target.yMm - h / 2), w_mm: w, h_mm: h, rotation_deg: 0 };
    const el =
      kind === "text"
        ? textElement(frame)
        : { id: newId(), frame, locked: false, zone: false, kind: { type: "Shape" as const, shape: "Rect" as const, fill: "#f6e3c5", stroke: null, corner_radius_mm: 0 } };
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, el] }));
    editor.selection = [el.id];
    if (kind === "text") editor.editingText = el.id;
  }
</script>

<svelte:window
  onpointerdown={(e) => {
    if (menuEl && !menuEl.contains(e.target as Node)) onclose();
  }}
  onkeydown={(e) => e.key === "Escape" && onclose()}
  onblur={onclose}
/>

<div class="menu" role="menu" bind:this={menuEl} style:left="{left}px" style:top="{top}px">
  {#if element}
    <button role="menuitem" disabled={element.locked} onclick={() => run(cutSelection)}><span>{fr.cut}</span><kbd>Ctrl+X</kbd></button>
    <button role="menuitem" onclick={() => run(copySelection)}><span>{fr.copy}</span><kbd>Ctrl+C</kbd></button>
    <button role="menuitem" disabled={!hasClipboard()} onclick={() => run(paste)}><span>{fr.paste}</span><kbd>Ctrl+V</kbd></button>
    <button role="menuitem" onclick={() => run(duplicateSelection)}><span>{fr.duplicate}</span><kbd>Ctrl+D</kbd></button>
    <hr />
    <button role="menuitem" onclick={() => run(() => bringSelection("front"))}><span>{fr.bringFront}</span></button>
    <button role="menuitem" onclick={() => run(() => bringSelection("back"))}><span>{fr.sendBack}</span></button>
    <hr />
    <button role="menuitem" onclick={() => run(toggleLockSelection)}><span>{editor.templateMode ? (element.locked ? fr.unfixCell : fr.fixCell) : element.locked ? fr.unlock : fr.lock}</span></button>
    <button role="menuitem" disabled={element.locked} onclick={() => run(deleteSelection)}><span>{fr.delete}</span><kbd>Suppr</kbd></button>
  {:else}
    <button role="menuitem" disabled={!hasClipboard()} onclick={() => run(paste)}><span>{fr.paste}</span><kbd>Ctrl+V</kbd></button>
    <button role="menuitem" onclick={() => run(() => addHere("text"))}><span>{fr.addTextHere}</span></button>
    <button role="menuitem" onclick={() => run(() => addHere("shape"))}><span>{fr.addShapeHere}</span></button>
  {/if}
</div>

<style>
  .menu {
    position: fixed;
    background: white;
    border: 1px solid var(--line);
    border-radius: 8px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.15);
    padding: 4px;
    min-width: 210px;
    z-index: 4000; /* au-dessus des poignées Moveable (3000) */
  }
  button {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    width: 100%;
    border: 0;
    background: transparent;
    text-align: left;
    padding: 5px 10px;
  }
  button:not(:disabled):hover {
    background: #eef5fc;
  }
  kbd {
    font: inherit;
    color: #aaa;
  }
  hr {
    border: 0;
    border-top: 1px solid var(--line);
    margin: 3px 0;
  }
</style>
