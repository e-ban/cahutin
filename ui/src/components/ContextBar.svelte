<script lang="ts">
  // Barre flottante au-dessus de l'élément sélectionné : seulement ce qui le concerne.
  // Cachée pendant un geste à la souris, un recadrage et en mode modèle.
  import { tick } from "svelte";
  import ColorPicker from "./ColorPicker.svelte";
  import { alignFrames, distributeFrames, type AlignMode } from "../lib/align";
  import { placeContextBar, selectionBox } from "../lib/contextBar";
  import { editor } from "../lib/docStore.svelte";
  import { deleteSelection } from "../lib/editActions";
  import { endPreview, previewPhoto, previewShape, updatePhoto, updateShape } from "../lib/elementEdit";
  import { FONT_SIZES, fonts } from "../lib/fonts.svelte";
  import { fr } from "../lib/i18n/fr";
  import { ALIGNS, isMarkActive, setAlign, setColor, setFont, setSize, toggleMark } from "../lib/textFormat.svelte";
  import type { Frame } from "../lib/types";

  let { pageEl, stageEl }: { pageEl: HTMLElement | undefined; stageEl: HTMLElement | undefined } = $props();

  const ROTATION_HANDLE_GAP = 44;

  type Pop = "corners" | "border" | "color" | "align" | "fill" | "stroke" | "alignMany" | "distribute";
  let pop = $state<Pop | null>(null);
  let barEl = $state<HTMLElement>();
  let pos = $state<{ left: number; top: number } | null>(null);

  const inGrid = (id: string) => !!editor.page.grid?.cells.some((c) => c.element_id === id);
  const selected = $derived(editor.selected);
  const many = $derived(editor.selection.length > 1);
  // En mode modèle, une case de grille a sa propre barre (CellBar).
  const cellInTemplate = $derived(editor.templateMode && editor.selection.some((id) => inGrid(id)));
  const visible = $derived(editor.selection.length > 0 && !editor.gesture && !editor.cropping && !cellInTemplate && !editor.printing);

  $effect(() => {
    void editor.selection;
    pop = null;
  });

  async function reposition() {
    await tick();
    if (!visible || !pageEl || !stageEl || !barEl) {
      pos = null;
      return;
    }
    const box = selectionBox(pageEl, editor.selection);
    if (!box) {
      pos = null;
      return;
    }
    const s = stageEl.getBoundingClientRect();
    // La poignée de rotation (Moveable) dépasse au-dessus d'un élément libre sélectionné seul.
    const e = selected;
    const handles = !!e && !e.locked && !inGrid(e.id) && editor.editingText !== e.id;
    pos = placeContextBar(
      box,
      { width: barEl.offsetWidth, height: barEl.offsetHeight },
      { left: s.left, top: s.top, width: s.width, height: s.height },
      handles ? ROTATION_HANDLE_GAP : undefined,
    );
  }

  $effect(() => {
    void [editor.doc, editor.selection, editor.effectiveZoom, editor.drawer, editor.inspectorOpen, editor.editingText, visible, barEl];
    reposition();
  });

  $effect(() => {
    const el = stageEl;
    if (!el) return;
    const onScroll = () => reposition();
    el.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", onScroll);
    return () => {
      el.removeEventListener("scroll", onScroll);
      window.removeEventListener("resize", onScroll);
    };
  });

  const toggle = (p: Pop) => (pop = pop === p ? null : p);

  // Garde le focus de l'éditeur de texte pendant un clic sur la barre.
  const keepFocus = (e: PointerEvent) => {
    if (!(e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement)) e.preventDefault();
  };

  const removeSelected = deleteSelection;

  /** Éléments libres de la sélection (ni verrouillés, ni cases de grille). */
  function movable() {
    return editor.page.elements.filter((e) => editor.selection.includes(e.id) && !e.locked && !inGrid(e.id));
  }

  function applyFrames(fn: (frames: Frame[]) => Frame[]) {
    const els = movable();
    const frames = fn(els.map((e) => e.frame));
    const byId = new Map(els.map((e, i) => [e.id, frames[i]]));
    editor.updatePage((p) => ({ ...p, elements: p.elements.map((e) => (byId.has(e.id) ? { ...e, frame: byId.get(e.id)! } : e)) }));
    pop = null;
  }

  const alignModes: [AlignMode, string][] = [
    ["left", fr.alignLeft],
    ["hcenter", fr.alignHCenter],
    ["right", fr.alignRight],
    ["top", fr.alignTop],
    ["vmiddle", fr.alignVMiddle],
    ["bottom", fr.alignBottom],
  ];

  function openPhotos() {
    editor.drawer = "photos";
  }
</script>

{#if visible}
  <div
    class="ctx"
    role="toolbar"
    tabindex="-1"
    aria-label="Barre de l'élément"
    bind:this={barEl}
    style:left={pos ? `${pos.left}px` : "-9999px"}
    style:top={pos ? `${pos.top}px` : "-9999px"}
    onpointerdown={keepFocus}
  >
    {#if many}
      <div class="wrap">
        <button onclick={() => toggle("alignMany")} disabled={movable().length < 2}>{fr.alignMenu} ▾</button>
        {#if pop === "alignMany"}
          <div class="pop list">
            {#each alignModes as [mode, label] (mode)}<button onclick={() => applyFrames((f) => alignFrames(f, mode))}>{label}</button>{/each}
          </div>
        {/if}
      </div>
      <div class="wrap">
        <button onclick={() => toggle("distribute")} disabled={movable().length < 3}>{fr.distribute} ▾</button>
        {#if pop === "distribute"}
          <div class="pop list">
            <button onclick={() => applyFrames((f) => distributeFrames(f, "h"))}>{fr.distributeH}</button>
            <button onclick={() => applyFrames((f) => distributeFrames(f, "v"))}>{fr.distributeV}</button>
          </div>
        {/if}
      </div>
      <span class="sep"></span>
      <button onclick={removeSelected}>🗑 {fr.delete}</button>
    {:else if selected?.kind.type === "Photo"}
      {@const ph = selected.kind}
      {#if !ph.asset}
        <button class="main" onclick={openPhotos}>🖼 {fr.choosePhoto}</button>
      {:else}
        <button onclick={() => (editor.cropping = selected.id)}>✂ {fr.crop}</button>
        <div class="wrap">
          <button class:on={pop === "corners"} onclick={() => toggle("corners")}>◜ {fr.corners}</button>
          {#if pop === "corners"}
            <div class="pop">
              <span>{ph.corner_radius_mm} mm</span>
              <input type="range" min="0" max="20" step="1" value={ph.corner_radius_mm} oninput={(e) => previewPhoto({ corner_radius_mm: Number(e.currentTarget.value) })} onchange={endPreview} />
            </div>
          {/if}
        </div>
        <div class="wrap">
          <button class:on={!!ph.border} onclick={() => toggle("border")}>▢ {fr.border}</button>
          {#if pop === "border"}
            <div class="pop">
              <label class="check">
                <input type="checkbox" checked={!!ph.border} onchange={(e) => updatePhoto({ border: e.currentTarget.checked ? { width_mm: 1, color: "#ffffff" } : null })} />
                {fr.border}
              </label>
              {#if ph.border}
                <ColorPicker value={ph.border.color} onchange={(c) => updatePhoto({ border: { width_mm: ph.border!.width_mm, color: c ?? "#ffffff" } })} />
              {/if}
            </div>
          {/if}
        </div>
        <button class:on={ph.shadow} aria-pressed={ph.shadow} onclick={() => updatePhoto({ shadow: !ph.shadow })}>◩ {fr.shadow}</button>
        <button onclick={openPhotos}>⇄ {fr.replace}</button>
        {#if !selected.locked}<button title={fr.delete} aria-label={fr.delete} onclick={removeSelected}>🗑</button>{/if}
      {/if}
    {:else if selected?.kind.type === "Text"}
      {@const t = selected.kind}
      {@const family = fonts.families.find((f) => f.name === t.style.font)}
      <select class="font" title={fr.font} value={t.style.font} onchange={(e) => setFont(e.currentTarget.value)}>
        {#each fonts.families as f (f.name)}<option value={f.name} style:font-family={`"${f.name}"`}>{f.name}</option>{/each}
      </select>
      <select title={fr.size} value={String(t.style.size_pt)} onchange={(e) => setSize(Number(e.currentTarget.value))}>
        {#each FONT_SIZES as s (s)}<option value={String(s)}>{s}</option>{/each}
      </select>
      <span class="sep"></span>
      <button title={fr.bold} class:on={isMarkActive("bold", t)} disabled={family && !family.has_bold} onclick={() => toggleMark("bold")}><b>G</b></button>
      <button title={fr.italic} class:on={isMarkActive("italic", t)} disabled={family && !family.has_italic} onclick={() => toggleMark("italic")}><i>I</i></button>
      <button title={fr.underline} class:on={isMarkActive("underline", t)} onclick={() => toggleMark("underline")}><u>S</u></button>
      <div class="wrap">
        <button title={fr.color} aria-label={fr.color} onclick={() => toggle("color")}><span class="dot" style:background={t.style.color}></span></button>
        {#if pop === "color"}<div class="pop"><ColorPicker value={t.style.color} onchange={(c) => (setColor(c), (pop = null))} /></div>{/if}
      </div>
      <div class="wrap">
        <button title={fr.alignment} onclick={() => toggle("align")}>{ALIGNS.find(([a]) => a === t.style.align)?.[1] ?? "≡"} ▾</button>
        {#if pop === "align"}
          <div class="pop row">
            {#each ALIGNS as [a, icon] (a)}<button class:on={t.style.align === a} onclick={() => (setAlign(a), (pop = null))}>{icon}</button>{/each}
          </div>
        {/if}
      </div>
    {:else if selected?.kind.type === "Shape"}
      {@const s = selected.kind}
      <div class="wrap">
        <button onclick={() => toggle("fill")}><span class="dot" style:background={s.fill ?? "transparent"}></span> {fr.fill}</button>
        {#if pop === "fill"}<div class="pop"><ColorPicker value={s.fill} allowNone onchange={(c) => updateShape({ fill: c })} /></div>{/if}
      </div>
      <div class="wrap">
        <button class:on={!!s.stroke} onclick={() => toggle("stroke")}>▢ {fr.stroke}</button>
        {#if pop === "stroke"}
          <div class="pop">
            <span>{s.stroke ? `${s.stroke.width_mm} mm` : "—"}</span>
            <input
              type="range"
              min="0"
              max="3"
              step="0.5"
              value={s.stroke?.width_mm ?? 0}
              oninput={(e) => {
                const w = Number(e.currentTarget.value);
                previewShape({ stroke: w === 0 ? null : { width_mm: w, color: s.stroke?.color ?? "#999999" } });
              }}
              onchange={endPreview}
            />
            {#if s.stroke}<ColorPicker value={s.stroke.color} onchange={(c) => updateShape({ stroke: { width_mm: s.stroke!.width_mm, color: c ?? "#999999" } })} />{/if}
          </div>
        {/if}
      </div>
      {#if s.shape === "Rect"}
        <div class="wrap">
          <button onclick={() => toggle("corners")}>◜ {fr.corners}</button>
          {#if pop === "corners"}
            <div class="pop">
              <span>{s.corner_radius_mm} mm</span>
              <input type="range" min="0" max="20" step="1" value={s.corner_radius_mm} oninput={(e) => previewShape({ corner_radius_mm: Number(e.currentTarget.value) })} onchange={endPreview} />
            </div>
          {/if}
        </div>
      {/if}
      {#if !selected.locked}<button title={fr.delete} aria-label={fr.delete} onclick={removeSelected}>🗑</button>{/if}
    {/if}
    {#if !many}
      <span class="sep"></span>
      <button title={fr.more} aria-label={fr.more} onclick={() => (editor.inspectorOpen = true)}>⋯</button>
    {/if}
  </div>
{/if}

<style>
  .ctx {
    position: fixed;
    display: flex;
    gap: 2px;
    align-items: center;
    background: white;
    border: 1px solid var(--line);
    border-radius: 9px;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.12);
    padding: 3px;
    white-space: nowrap;
    z-index: 3500; /* au-dessus des poignées Moveable (3000), sous les menus */
  }
  .ctx > button,
  .wrap > button {
    border-color: transparent;
    background: transparent;
    padding: 4px 8px;
  }
  .ctx > button:not(:disabled):hover,
  .wrap > button:not(:disabled):hover {
    background: #f3efe8;
  }
  .on {
    background: #e8f1fb !important;
    color: var(--accent);
  }
  .main {
    color: var(--accent);
    font-weight: 600;
  }
  select {
    padding: 3px 4px;
  }
  .font {
    max-width: 150px;
  }
  .sep {
    width: 1px;
    align-self: stretch;
    background: var(--line);
    margin: 3px 2px;
  }
  .dot {
    display: inline-block;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 1px solid #ccc;
    vertical-align: middle;
  }
  .wrap {
    position: relative;
  }
  .pop {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    background: white;
    border: 1px solid var(--line);
    border-radius: 8px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.15);
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 180px;
    white-space: normal;
  }
  .pop.row {
    flex-direction: row;
    min-width: 0;
  }
  .pop.list {
    padding: 4px;
    gap: 0;
    min-width: 150px;
  }
  .pop.list button {
    border: 0;
    background: transparent;
    text-align: left;
  }
  .pop.list button:hover {
    background: #eef5fc;
  }
  .check {
    display: flex;
    gap: 6px;
    align-items: center;
  }
</style>
