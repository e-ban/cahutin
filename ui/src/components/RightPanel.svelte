<script lang="ts">
  import ColorPicker from "./ColorPicker.svelte";
  import { editor } from "../lib/docStore.svelte";
  import { fr } from "../lib/i18n/fr";
  import type { Element, ShapeProps } from "../lib/types";

  const selected = $derived(editor.selected);

  function update(fn: (e: Element) => Element) {
    if (selected) editor.updateElement(selected.id, fn);
  }

  function updateShape(patch: Partial<ShapeProps>) {
    update((e) => (e.kind.type === "Shape" ? { ...e, kind: { ...e.kind, ...patch } } : e));
  }
</script>

<aside class="right">
  {#if !selected}
    <p class="hint">{fr.noSelection}</p>
  {:else}
    <h3>{selected.kind.type === "Photo" ? fr.photo : selected.kind.type === "Text" ? fr.text : fr.shape}</h3>

    {#if selected.kind.type === "Shape"}
      {@const s = selected.kind}
      <div class="field">
        <span>Remplissage</span>
        <ColorPicker value={s.fill} allowNone onchange={(c) => updateShape({ fill: c })} />
      </div>
      <div class="field">
        <span>Trait : {s.stroke ? `${s.stroke.width_mm} mm` : "aucun"}</span>
        <input
          type="range"
          min="0"
          max="3"
          step="0.5"
          value={s.stroke?.width_mm ?? 0}
          oninput={(e) => {
            const w = Number(e.currentTarget.value);
            updateShape({ stroke: w === 0 ? null : { width_mm: w, color: s.stroke?.color ?? "#999999" } });
          }}
        />
        {#if s.stroke}
          <ColorPicker value={s.stroke.color} onchange={(c) => updateShape({ stroke: { width_mm: s.stroke!.width_mm, color: c ?? "#999999" } })} />
        {/if}
      </div>
      {#if s.shape === "Rect"}
        <div class="field">
          <span>Coins arrondis : {s.corner_radius_mm} mm</span>
          <input type="range" min="0" max="20" step="1" value={s.corner_radius_mm} oninput={(e) => updateShape({ corner_radius_mm: Number(e.currentTarget.value) })} />
        </div>
      {/if}
    {/if}

    <div class="actions">
      <button onclick={() => update((e) => ({ ...e, locked: !e.locked }))}>{selected.locked ? `🔓 ${fr.unlock}` : `🔒 ${fr.lock}`}</button>
      {#if !selected.locked}
        <button onclick={() => editor.updatePage((p) => ({ ...p, elements: p.elements.filter((e) => e.id !== selected.id) }))}>🗑 {fr.delete}</button>
      {/if}
    </div>
  {/if}
</aside>

<style>
  .right {
    grid-area: right;
    background: var(--panel);
    border-left: 1px solid var(--line);
    overflow-y: auto;
    padding: 12px;
  }
  .hint {
    color: var(--muted);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 14px;
  }
  .actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    margin-top: 18px;
  }
</style>
