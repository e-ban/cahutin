<script lang="ts">
  import ColorPicker from "./ColorPicker.svelte";
  import { editor } from "../lib/docStore.svelte";
  import { fr } from "../lib/i18n/fr";
  import { FONT_SIZES, fonts } from "../lib/fonts.svelte";
  import { alignToCss } from "../lib/richtext";
  import { textEditing } from "../lib/textEditing.svelte";
  import type { Align, Element, PhotoProps, Run, ShapeProps, TextProps, TextStyle } from "../lib/types";

  const selected = $derived(editor.selected);

  function update(fn: (e: Element) => Element) {
    if (selected) editor.updateElement(selected.id, fn);
  }

  // Curseurs : aperçu en direct pendant le glissement, un seul pas d'annulation au relâchement.
  function previewElement(fn: (e: Element) => Element) {
    if (!selected) return;
    const id = selected.id;
    const doc = editor.doc;
    editor.preview({
      ...doc,
      pages: doc.pages.map((p, i) => (i === editor.pageIndex ? { ...p, elements: p.elements.map((e) => (e.id === id ? fn(e) : e)) } : p)),
    });
  }

  function previewPhoto(patch: Partial<PhotoProps>) {
    previewElement((e) => (e.kind.type === "Photo" ? { ...e, kind: { ...e.kind, ...patch } } : e));
  }

  function previewShape(patch: Partial<ShapeProps>) {
    previewElement((e) => (e.kind.type === "Shape" ? { ...e, kind: { ...e.kind, ...patch } } : e));
  }

  const endPreview = () => editor.endPreview();

  function updatePhoto(patch: Partial<PhotoProps>) {
    update((e) => (e.kind.type === "Photo" ? { ...e, kind: { ...e.kind, ...patch } } : e));
  }

  // Texte : pendant l'édition, la mise en forme s'applique à la sélection ; sinon à toute la zone.
  const tip = $derived(selected && editor.editingText === selected.id ? textEditing.current : null);

  function updateText(fn: (t: TextProps) => TextProps) {
    update((e) => (e.kind.type === "Text" ? { ...e, kind: { type: "Text", ...fn(e.kind) } } : e));
  }

  function setStyle(patch: Partial<TextStyle>) {
    updateText((t) => ({ ...t, style: { ...t.style, ...patch } }));
  }

  function setFont(font: string) {
    if (tip) tip.chain().focus().setFontFamily(font).run();
    else setStyle({ font });
  }

  function setSize(pt: number) {
    if (tip) tip.chain().focus().setFontSize(`${pt}pt`).run();
    else setStyle({ size_pt: pt });
  }

  function setColor(color: string | null) {
    if (!color) return;
    if (tip) tip.chain().focus().setColor(color).run();
    else setStyle({ color });
  }

  function setAlign(align: Align) {
    if (tip) tip.chain().focus().setTextAlign(alignToCss(align)).run();
    else updateText((t) => ({ ...t, style: { ...t.style, align }, content: { paragraphs: t.content.paragraphs.map((p) => ({ ...p, align: null })) } }));
  }

  function toggle(mark: "bold" | "italic" | "underline") {
    if (tip) {
      tip.chain().focus().toggleMark(mark).run();
      return;
    }
    updateText((t) => {
      const all = t.content.paragraphs.flatMap((p) => p.runs);
      const on = !all.every((r: Run) => r[mark]);
      return { ...t, content: { paragraphs: t.content.paragraphs.map((p) => ({ ...p, runs: p.runs.map((r) => ({ ...r, [mark]: on })) })) } };
    });
  }

  function isActive(mark: "bold" | "italic" | "underline", t: TextProps): boolean {
    void textEditing.tick;
    if (tip) return tip.isActive(mark);
    const runs = t.content.paragraphs.flatMap((p) => p.runs);
    return runs.length > 0 && runs.every((r) => r[mark]);
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

    {#if selected.kind.type === "Photo"}
      {@const ph = selected.kind}
      {#if ph.asset}
        <p class="hint">Double-cliquez sur la photo pour la recadrer (glisser, molette), puis Échap.</p>
      {/if}
      <div class="field">
        <span>Coins arrondis : {ph.corner_radius_mm} mm</span>
        <input type="range" min="0" max="20" step="1" value={ph.corner_radius_mm} oninput={(e) => previewPhoto({ corner_radius_mm: Number(e.currentTarget.value) })} onchange={endPreview} />
      </div>
      <div class="field">
        <label class="check">
          <input
            type="checkbox"
            checked={!!ph.border}
            onchange={(e) => updatePhoto({ border: e.currentTarget.checked ? { width_mm: 1, color: "#ffffff" } : null })}
          />
          Bordure
        </label>
        {#if ph.border}
          <span>Épaisseur : {ph.border.width_mm} mm</span>
          <input
            type="range"
            min="0.5"
            max="3"
            step="0.5"
            value={ph.border.width_mm}
            oninput={(e) => previewPhoto({ border: { width_mm: Number(e.currentTarget.value), color: ph.border!.color } })}
            onchange={endPreview}
          />
          <ColorPicker value={ph.border.color} onchange={(c) => updatePhoto({ border: { width_mm: ph.border!.width_mm, color: c ?? "#ffffff" } })} />
        {/if}
      </div>
      <div class="field">
        <label class="check">
          <input type="checkbox" checked={ph.shadow} onchange={(e) => updatePhoto({ shadow: e.currentTarget.checked })} />
          Ombre
        </label>
      </div>
      {#if ph.asset}
        <button onclick={() => updatePhoto({ asset: null, crop: { scale: 1, center_x: 0.5, center_y: 0.5 } })}>Retirer la photo</button>
      {/if}
    {/if}

    {#if selected.kind.type === "Text"}
      {@const t = selected.kind}
      {@const family = fonts.families.find((f) => f.name === t.style.font)}
      {#if !tip}
        <p class="hint">Double-cliquez pour écrire. Sans édition, les réglages s'appliquent à toute la zone.</p>
      {/if}
      <div class="field">
        <span>Police</span>
        <select value={t.style.font} onchange={(e) => setFont(e.currentTarget.value)}>
          {#each fonts.families as f}<option value={f.name} style:font-family={`"${f.name}"`}>{f.name}</option>{/each}
        </select>
      </div>
      <div class="field">
        <span>Taille</span>
        <select value={String(t.style.size_pt)} onchange={(e) => setSize(Number(e.currentTarget.value))}>
          {#each FONT_SIZES as size}<option value={String(size)}>{size} pt</option>{/each}
        </select>
      </div>
      <div class="row">
        <button class:on={isActive("bold", t)} disabled={family && !family.has_bold} onpointerdown={(e) => e.preventDefault()} onclick={() => toggle("bold")}><b>G</b></button>
        <button class:on={isActive("italic", t)} disabled={family && !family.has_italic} onpointerdown={(e) => e.preventDefault()} onclick={() => toggle("italic")}><i>I</i></button>
        <button class:on={isActive("underline", t)} onpointerdown={(e) => e.preventDefault()} onclick={() => toggle("underline")}><u>S</u></button>
      </div>
      <div class="row">
        {#each [["Left", "⯇"], ["Center", "≡"], ["Right", "⯈"], ["Justify", "☰"]] as [a, icon]}
          <button title={a} class:on={t.style.align === a} onpointerdown={(e) => e.preventDefault()} onclick={() => setAlign(a as Align)}>{icon}</button>
        {/each}
      </div>
      <div class="field">
        <span>Couleur</span>
        <ColorPicker value={t.style.color} onchange={setColor} />
      </div>
      {#if selected.zone}
        <div class="field">
          <span>Texte d'invite (visible seulement à l'écran)</span>
          <input value={t.prompt ?? ""} onchange={(e) => updateText((x) => ({ ...x, prompt: e.currentTarget.value || null }))} />
        </div>
      {/if}
    {/if}

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
            previewShape({ stroke: w === 0 ? null : { width_mm: w, color: s.stroke?.color ?? "#999999" } });
          }}
          onchange={endPreview}
        />
        {#if s.stroke}
          <ColorPicker value={s.stroke.color} onchange={(c) => updateShape({ stroke: { width_mm: s.stroke!.width_mm, color: c ?? "#999999" } })} />
        {/if}
      </div>
      {#if s.shape === "Rect"}
        <div class="field">
          <span>Coins arrondis : {s.corner_radius_mm} mm</span>
          <input type="range" min="0" max="20" step="1" value={s.corner_radius_mm} oninput={(e) => previewShape({ corner_radius_mm: Number(e.currentTarget.value) })} onchange={endPreview} />
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
  .row {
    display: flex;
    gap: 6px;
    margin-bottom: 12px;
  }
  .on {
    background: #e3effb;
    border-color: var(--accent);
  }
  .check {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    margin-top: 18px;
  }
</style>
