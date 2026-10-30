<script lang="ts">
  // Barre flottante au-dessus de l'élément sélectionné : seulement ce qui le concerne.
  // Cachée pendant un geste à la souris, un recadrage et en mode modèle.
  import { tick } from "svelte";
  import AlignIcon from "./AlignIcon.svelte";
  import ColorPicker from "./ColorPicker.svelte";
  import StrokeStylePicker from "./StrokeStylePicker.svelte";
  import VAlignIcon from "./VAlignIcon.svelte";
  import FontPicker from "./FontPicker.svelte";
  import { alignFrames, distributeFrames, type AlignMode } from "../lib/align";
  import { placeContextBar, selectionBox } from "../lib/contextBar";
  import { editor } from "../lib/docStore.svelte";
  import { addCaption, deleteSelection, groupSelection, setFrameImage, toggleLockSelection, ungroupSelection } from "../lib/editActions";
  import { assetUrl } from "../lib/api";
  import { endPreview, previewPhoto, previewShape, previewTable, updateTable, previewTextFrame, previewTextStyle, updatePhoto, updateShape, updateTextFrame, updateTextStyle } from "../lib/elementEdit";
  import { textPadding } from "../lib/richtext";
  import { FONT_SIZES, fonts } from "../lib/fonts.svelte";
  import { fr } from "../lib/i18n/fr";
  import { isStrokeOnly } from "../lib/shapePaths";
  import { isTableCell, selectedTable } from "../lib/table";
  import { alignUnits, arrangeUnits, expandSelection, isLockedSelection } from "../lib/groups";
  import { blockFrieze } from "../lib/decor";
  import { cutMarks, type CutStyle } from "../lib/cutMarks";
  import { friezeMotifs } from "../lib/friezeMotifs.svelte";
  import { ALIGNS, VALIGNS, currentFont, currentSize, insertAtCursor, isMarkActive, setAlign, setColor, setFont, setSize, setVAlign, toggleMark } from "../lib/textFormat.svelte";
  import type { Frame } from "../lib/types";

  let { pageEl, stageEl }: { pageEl: HTMLElement | undefined; stageEl: HTMLElement | undefined } = $props();

  const ROTATION_HANDLE_GAP = 44;

  type Pop = "corners" | "border" | "color" | "align" | "fill" | "stroke" | "alignMany" | "distribute" | "frame" | "cut";
  let pop = $state<Pop | null>(null);
  let barEl = $state<HTMLElement>();
  let pos = $state<{ left: number; top: number } | null>(null);

  const inGrid = (id: string) => !!editor.page.grid?.cells.some((c) => c.element_id === id);
  // Sélection qui est exactement un groupe : on propose Dissocier ; sinon Grouper (hors cases de grille).
  const isGroup = $derived.by(() => {
    const groups = new Set(editor.page.elements.filter((e) => editor.selection.includes(e.id)).map((e) => e.group ?? null));
    const [g] = [...groups];
    return groups.size === 1 && !!g && editor.page.elements.filter((e) => e.group === g).length === editor.selection.length;
  });
  const canGroup = $derived(!editor.selection.some(inGrid));
  const selected = $derived(editor.selected);
  const many = $derived(editor.selection.length > 1);
  const table = $derived(selectedTable(editor.page, editor.selection));
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
    const handles = many
      ? !editor.selection.some(inGrid) && !isLockedSelection(editor.page, editor.selection)
      : !!e && !e.locked && !inGrid(e.id) && editor.editingText !== e.id;
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
  /** Blocs à aligner ou répartir : un groupe compte pour un. */
  const units = () => alignUnits(editor.page, editor.selection).length;

  function applyFrames(fn: (frames: Frame[]) => Frame[]) {
    editor.updatePage((p) => arrangeUnits(p, editor.selection, fn));
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

  /** Bloc seul et libre (ni case de grille, ni verrouillé, ni trait) : une frise peut l'entourer. */
  const canFrieze = $derived.by(() => {
    const e = selected;
    // Une case de tableau n'en a pas : la frise rejoindrait le groupe du tableau.
    if (many || !e || e.locked || inGrid(e.id) || isTableCell(editor.page, e.id)) return false;
    const k = e.kind;
    return k.type === "Text" || (k.type === "Photo" && !!k.asset) || (k.type === "Shape" && !isStrokeOnly(k.shape));
  });

  /** Repères de découpe : pas sur une case de grille ou de tableau, ni sur un bloc verrouillé. */
  const canCut = $derived(
    editor.selection.length > 0 && !editor.selection.some((id) => inGrid(id) || isTableCell(editor.page, id)) && !isLockedSelection(editor.page, editor.selection),
  );

  function setCutMarks(style: CutStyle | null) {
    editor.updatePage((p) => cutMarks(p, editor.selection, style));
    editor.focusedMember = null;
    editor.selection = expandSelection(editor.page, editor.selection.filter((id) => editor.page.elements.some((e) => e.id === id)));
    pop = null;
  }

  function addBlockFrieze() {
    const id = selected?.id;
    if (!id) return;
    if (friezeMotifs.codes.length === 0) {
      editor.drawer = "decor";
      return;
    }
    editor.updatePage((p) => blockFrieze(p, id, friezeMotifs.codes) ?? p);
    editor.focusedMember = null;
    editor.selection = expandSelection(editor.page, [id]);
  }

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
      {#if isGroup}
        <button class="main" title="Ctrl+Maj+G" onclick={ungroupSelection}>⊟ {fr.ungroup}</button>
      {:else}
        <button class="main" title="Ctrl+G" disabled={!canGroup} onclick={groupSelection}>⊞ {fr.group}</button>
      {/if}
      <span class="sep"></span>
      {#if table?.kind.type === "Table"}
        {@const t = table.kind}
        {@const id = table.id}
        <div class="wrap">
          <button class:on={!!t.border} onclick={() => toggle("stroke")}>▦ {fr.tableBorders}</button>
          {#if pop === "stroke"}
            <div class="pop">
              <span>{t.border ? `${t.border.width_mm} mm` : fr.noStroke}</span>
              <input
                type="range"
                min="0"
                max="3"
                step="0.5"
                value={t.border?.width_mm ?? 0}
                oninput={(e) => {
                  const w = Number(e.currentTarget.value);
                  previewTable(id, { border: w === 0 ? null : { width_mm: w, color: t.border?.color ?? "#333333", style: t.border?.style ?? null } });
                }}
                onchange={endPreview}
              />
              {#if t.border}
                <ColorPicker value={t.border.color} onchange={(c) => updateTable(id, { border: { ...t.border!, color: c ?? "#333333" } })} />
                <StrokeStylePicker value={t.border.style} onpick={(style) => updateTable(id, { border: { ...t.border!, style: style === "Solid" ? null : style } })} />
              {/if}
            </div>
          {/if}
        </div>
        <span class="sep"></span>
      {/if}
      <div class="wrap">
        <button onclick={() => toggle("alignMany")} disabled={units() < 2}>{fr.alignMenu} ▾</button>
        {#if pop === "alignMany"}
          <div class="pop list">
            {#each alignModes as [mode, label] (mode)}<button onclick={() => applyFrames((f) => alignFrames(f, mode))}>{label}</button>{/each}
          </div>
        {/if}
      </div>
      <div class="wrap">
        <button onclick={() => toggle("distribute")} disabled={units() < 3}>{fr.distribute} ▾</button>
        {#if pop === "distribute"}
          <div class="pop list">
            <button onclick={() => applyFrames((f) => distributeFrames(f, "h"))}>{fr.distributeH}</button>
            <button onclick={() => applyFrames((f) => distributeFrames(f, "v"))}>{fr.distributeV}</button>
          </div>
        {/if}
      </div>
      <span class="sep"></span>
      {#if isGroup}<button title={fr.lock} aria-label={fr.lock} onclick={toggleLockSelection}>🔒</button>{/if}
      <button onclick={removeSelected}>🗑 {fr.delete}</button>
    {:else if selected?.kind.type === "Photo"}
      {@const ph = selected.kind}
      {#if !ph.asset}
        <button class="main" onclick={openPhotos}>🖼 {fr.choosePhoto}</button>
        <button onclick={() => addCaption(selected.id)}>✎ {fr.caption}</button>
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
                <ColorPicker value={ph.border.color} onchange={(c) => updatePhoto({ border: { ...ph.border!, color: c ?? "#ffffff" } })} />
                <StrokeStylePicker value={ph.border.style} onpick={(style) => updatePhoto({ border: { ...ph.border!, style: style === "Solid" ? null : style } })} />
              {/if}
            </div>
          {/if}
        </div>
        <button class:on={ph.shadow} aria-pressed={ph.shadow} onclick={() => updatePhoto({ shadow: !ph.shadow })}>◩ {fr.shadow}</button>
        <button onclick={openPhotos}>⇄ {fr.replace}</button>
        <button onclick={() => addCaption(selected.id)}>✎ {fr.caption}</button>
        {#if !selected.locked}<button title={fr.delete} aria-label={fr.delete} onclick={removeSelected}>🗑</button>{/if}
      {/if}
    {:else if selected?.kind.type === "Text"}
      {@const t = selected.kind}
      {@const family = fonts.families.find((f) => f.name === currentFont(t))}
      <FontPicker value={currentFont(t)} onpick={setFont} compact />
      <select title={fr.size} value={String(currentSize(t))} onchange={(e) => setSize(Number(e.currentTarget.value))}>
        {#each FONT_SIZES as s (s)}<option value={String(s)}>{s}</option>{/each}
      </select>
      <span class="sep"></span>
      <button title={fr.bold} class:on={isMarkActive("bold", t)} disabled={family && !family.has_bold} onclick={() => toggleMark("bold")}><b>G</b></button>
      <button title={fr.italic} class:on={isMarkActive("italic", t)} disabled={family && !family.has_italic} onclick={() => toggleMark("italic")}><i>I</i></button>
      <button title={fr.underline} class:on={isMarkActive("underline", t)} onclick={() => toggleMark("underline")}><u>S</u></button>
      {#if editor.editingText === selected.id}
        <button title={fr.insertFirstNameHint} onclick={() => insertAtCursor("{prénom}")}>＋ {fr.insertFirstName}</button>
      {/if}
      <button title={fr.outlineHint} class:on={!!t.style.outline} aria-pressed={!!t.style.outline} onclick={() => updateTextStyle({ outline: !t.style.outline })}><span class="hollow-a">A</span></button>
      <div class="wrap">
        <button title={fr.color} aria-label={fr.color} onclick={() => toggle("color")}><span class="dot" style:background={t.style.color}></span></button>
        {#if pop === "color"}<div class="pop"><ColorPicker value={t.style.color} onchange={(c) => (setColor(c), (pop = null))} /></div>{/if}
      </div>
      <div class="wrap">
        <button class="iconbtn" title={fr.alignment} aria-label={fr.alignment} onclick={() => toggle("align")}><AlignIcon align={t.style.align} /> ▾</button>
        {#if pop === "align"}
          <div class="pop grid">
            <div class="row">
              {#each ALIGNS as a (a)}
                <button class:on={t.style.align === a} title={fr.textAlign[a]} aria-label={fr.textAlign[a]} onclick={() => setAlign(a)}><AlignIcon align={a} /></button>
              {/each}
            </div>
            <div class="row">
              {#each VALIGNS as v (v)}
                <button class:on={(t.style.valign ?? "Top") === v} title={fr.textVAlign[v]} aria-label={fr.textVAlign[v]} onclick={() => setVAlign(v)}><VAlignIcon valign={v} /></button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
      <div class="wrap">
        <button class:on={!!t.frame} onclick={() => toggle("frame")}>▭ {fr.textFrame}</button>
        {#if pop === "frame"}
          {@const f = t.frame}
          <div class="pop frame">
            <span>{fr.frameFill}</span>
            <ColorPicker value={f?.fill ?? null} allowNone onchange={(c) => updateTextFrame({ fill: c })} />
            <span>{fr.border} : {f?.stroke ? `${f.stroke.width_mm} mm` : fr.none}</span>
            <input
              type="range"
              min="0"
              max="3"
              step="0.5"
              value={f?.stroke?.width_mm ?? 0}
              oninput={(e) => {
                const w = Number(e.currentTarget.value);
                previewTextFrame({ stroke: w === 0 ? null : { width_mm: w, color: f?.stroke?.color ?? "#333333", style: f?.stroke?.style ?? null } });
              }}
              onchange={endPreview}
            />
            {#if f?.stroke}
              <ColorPicker value={f.stroke.color} onchange={(c) => updateTextFrame({ stroke: { ...f.stroke!, color: c ?? "#333333" } })} />
              <StrokeStylePicker value={f.stroke.style} onpick={(style) => updateTextFrame({ stroke: { ...f.stroke!, style: style === "Solid" ? null : style } })} />
            {/if}
            {#if f}
              <span>{fr.corners} : {f.corner_radius_mm} mm</span>
              <input type="range" min="0" max="20" step="1" value={f.corner_radius_mm} oninput={(e) => previewTextFrame({ corner_radius_mm: Number(e.currentTarget.value) })} onchange={endPreview} />
            {/if}
            <span>{fr.padding} : {textPadding(t.style)} mm</span>
            <input type="range" min="0" max="10" step="0.5" value={textPadding(t.style)} oninput={(e) => previewTextStyle({ padding_mm: Number(e.currentTarget.value) })} onchange={endPreview} />
            <span>{fr.frameImage}</span>
            {#if editor.photos.length === 0 && !f?.image}
              <button class="link" onclick={openPhotos}>{fr.frameImageHint}</button>
            {:else}
              <div class="frame-images">
                <button class="none" class:on={!f?.image} onclick={() => setFrameImage(null)}>{fr.frameImageNone}</button>
                {#each editor.photos as p (p.meta.id)}
                  <button class:on={f?.image === p.meta.id} onclick={() => setFrameImage(p.meta)}><img src={assetUrl(p.meta.id, true)} alt="" draggable="false" /></button>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {:else if selected?.kind.type === "Image"}
      <button onclick={() => (editor.drawer = "decor")}>⇄ {fr.replace}</button>
      {#if !selected.locked}<button title={fr.delete} aria-label={fr.delete} onclick={removeSelected}>🗑</button>{/if}
    {:else if selected?.kind.type === "Shape"}
      {@const s = selected.kind}
      {#if !isStrokeOnly(s.shape)}
      <div class="wrap">
        <button onclick={() => toggle("fill")}><span class="dot" style:background={s.fill ?? "transparent"}></span> {fr.fill}</button>
        {#if pop === "fill"}<div class="pop"><ColorPicker value={s.fill} allowNone onchange={(c) => updateShape({ fill: c })} /></div>{/if}
      </div>
      {/if}
      <div class="wrap">
        <button class:on={!!s.stroke} onclick={() => toggle("stroke")}>▢ {fr.stroke}</button>
        {#if pop === "stroke"}
          <div class="pop">
            <span>{s.stroke ? `${s.stroke.width_mm} mm` : fr.noStroke}</span>
            <input
              type="range"
              min={isStrokeOnly(s.shape) ? 0.5 : 0}
              max="3"
              step="0.5"
              value={s.stroke?.width_mm ?? 0}
              oninput={(e) => {
                const w = Number(e.currentTarget.value);
                previewShape({ stroke: w === 0 ? null : { width_mm: w, color: s.stroke?.color ?? "#999999", style: s.stroke?.style ?? null } });
              }}
              onchange={endPreview}
            />
            {#if s.stroke}
              <ColorPicker value={s.stroke.color} onchange={(c) => updateShape({ stroke: { ...s.stroke!, color: c ?? "#999999" } })} />
              <StrokeStylePicker value={s.stroke.style} onpick={(style) => updateShape({ stroke: { ...s.stroke!, style: style === "Solid" ? null : style } })} />
            {/if}
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
    {#if canFrieze}
      <button title={fr.blockFriezeHint} onclick={addBlockFrieze}>❀ {fr.blockFrieze}</button>
    {/if}
    {#if canCut}
      <div class="wrap">
        <button title={fr.cutMarksHint} onclick={() => toggle("cut")}>✂ {fr.cutMarks} ▾</button>
        {#if pop === "cut"}
          <div class="pop list">
            <button onclick={() => setCutMarks("corners")}>⌜ ⌝ {fr.cutCorners}</button>
            <button onclick={() => setCutMarks("dashed")}>✂ ┄ {fr.cutDashed}</button>
            <button onclick={() => setCutMarks(null)}>{fr.cutNone}</button>
          </div>
        {/if}
      </div>
    {/if}
    {#if !many}
      <span class="sep"></span>
      <button title={fr.more} aria-label={fr.more} onclick={() => (editor.inspectorOpen = true)}>⋯</button>
    {/if}
  </div>
{/if}

<style>
  .hollow-a {
    -webkit-text-fill-color: transparent;
    -webkit-text-stroke: 1px currentColor;
    font-weight: 700;
  }
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
  .iconbtn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .main {
    color: var(--accent);
    font-weight: 600;
  }
  select {
    padding: 3px 4px;
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
  .pop.frame {
    min-width: 230px;
    gap: 6px;
  }
  .pop.frame > span {
    font-size: 12px;
    color: var(--muted);
  }
  .frame-images {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 4px;
    max-height: 132px;
    overflow-y: auto;
  }
  .frame-images button {
    padding: 2px;
    aspect-ratio: 1;
  }
  .frame-images img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .frame-images .none {
    font-size: 11px;
  }
  .link {
    border: 0;
    background: none;
    color: var(--accent);
    text-align: left;
    padding: 0;
    white-space: normal;
    font-size: 12px;
  }
  .pop.grid {
    min-width: 0;
  }
  .pop .row {
    display: flex;
    gap: 2px;
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
