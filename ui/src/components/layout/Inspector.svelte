<script lang="ts">
  import AlignIcon from "../AlignIcon.svelte";
  import ColorPicker from "../ColorPicker.svelte";
  import FontPicker from "../FontPicker.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { isStrokeOnly } from "../../lib/shapePaths";
  import { FONT_SIZES, fonts } from "../../lib/fonts.svelte";
  import { ALIGNS, activeTip, isMarkActive, setAlign, setColor, setFont, setSize, toggleMark, updatePrompt } from "../../lib/textFormat.svelte";
  import { endPreview, previewPhoto, previewShape, updatePhoto, updateSelected, updateShape } from "../../lib/elementEdit";
  import { deleteSelection, setSelectedFrame, toggleLockSelection, ungroupSelection } from "../../lib/editActions";
  import { boundsOf, isLockedSelection } from "../../lib/groups";
  import type { Frame } from "../../lib/types";

  const selected = $derived(editor.selected);

  const update = updateSelected;

  // Groupe sélectionné : sa position déplace tous ses éléments.
  const groupIds = $derived.by(() => {
    const els = editor.page.elements.filter((e) => editor.selection.includes(e.id));
    const g = els[0]?.group;
    return els.length > 1 && g && els.every((e) => e.group === g) ? els.map((e) => e.id) : null;
  });
  const groupBox = $derived(groupIds ? boundsOf(editor.page.elements.filter((e) => groupIds.includes(e.id)).map((e) => e.frame)) : null);
  const groupLocked = $derived(!!groupIds && isLockedSelection(editor.page, groupIds));

  function moveGroup(axis: "x" | "y", raw: string) {
    const v = Number(raw.replace(",", "."));
    if (!groupIds || !groupBox || !Number.isFinite(v)) return;
    const d = v - (axis === "x" ? groupBox.x_mm : groupBox.y_mm);
    const key = axis === "x" ? "x_mm" : "y_mm";
    editor.updatePage((p) => ({
      ...p,
      elements: p.elements.map((e) => (groupIds.includes(e.id) ? { ...e, frame: { ...e.frame, [key]: Math.round((e.frame[key] + d) * 10) / 10 } } : e)),
    }));
  }

  const tip = $derived(selected ? activeTip() : null);

  // Position et taille exactes : une case de grille se règle en mode modèle, un élément verrouillé ne bouge pas.
  const frameFixed = $derived(!!selected && (selected.locked || !!editor.page.grid?.cells.some((c) => c.element_id === selected.id)));

  function setFrame(key: keyof Frame, raw: string) {
    setSelectedFrame(key, Number(raw.replace(",", ".")));
  }

</script>

{#if !editor.inspectorOpen}
  <button class="tab" title={fr.openInspector} aria-label={fr.openInspector} onclick={() => (editor.inspectorOpen = true)}>⚙ {fr.inspector} ◂</button>
{:else}
<aside class="right" aria-label={fr.inspector}>
  <header>
    <h3>{fr.inspector}</h3>
    <button class="close" title={fr.closeInspector} aria-label={fr.closeInspector} onclick={() => (editor.inspectorOpen = false)}>▸</button>
  </header>
  {#if groupIds}
    <h3>{fr.groupOf(groupIds.length)}</h3>
    <div class="field">
      <span>{fr.groupPosition}</span>
      <div class="frame">
        <label>{fr.posX}<input type="number" step="0.5" disabled={groupLocked} value={Math.round(groupBox!.x_mm * 10) / 10} onchange={(e) => moveGroup("x", e.currentTarget.value)} /></label>
        <label>{fr.posY}<input type="number" step="0.5" disabled={groupLocked} value={Math.round(groupBox!.y_mm * 10) / 10} onchange={(e) => moveGroup("y", e.currentTarget.value)} /></label>
      </div>
    </div>
    <div class="actions">
      <button onclick={ungroupSelection}>⊟ {fr.ungroup}</button>
      <button onclick={toggleLockSelection}>{groupLocked ? `🔓 ${fr.unlock}` : `🔒 ${fr.lock}`}</button>
      <button onclick={deleteSelection}>🗑 {fr.delete}</button>
    </div>
  {:else if !selected}
    <p class="hint">{fr.noSelection}</p>
  {:else}
    <h3>{selected.kind.type === "Photo" ? fr.photo : selected.kind.type === "Text" ? fr.text : selected.kind.type === "Image" ? fr.picture : fr.shape}</h3>

    <div class="field">
      <span>{fr.position}</span>
      <div class="frame">
        {#each [["x_mm", fr.posX], ["y_mm", fr.posY], ["w_mm", fr.width], ["h_mm", fr.height]] as [key, label] (key)}
          <label>
            {label}
            <input type="number" step="0.5" disabled={frameFixed} value={selected.frame[key as keyof Frame]} onchange={(e) => setFrame(key as keyof Frame, e.currentTarget.value)} />
          </label>
        {/each}
      </div>
      <label class="rot">
        {fr.rotation}
        <input type="number" step="1" disabled={frameFixed} value={selected.frame.rotation_deg} onchange={(e) => setFrame("rotation_deg", e.currentTarget.value)} />
      </label>
      {#if frameFixed && !selected.locked}<small class="hint">{fr.gridLockedFrame}</small>{/if}
    </div>

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
        <FontPicker value={t.style.font} onpick={setFont} />
      </div>
      <div class="field">
        <span>Taille</span>
        <select value={String(t.style.size_pt)} onchange={(e) => setSize(Number(e.currentTarget.value))}>
          {#each FONT_SIZES as size}<option value={String(size)}>{size} pt</option>{/each}
        </select>
      </div>
      <div class="row">
        <button class:on={isMarkActive("bold", t)} disabled={family && !family.has_bold} onpointerdown={(e) => e.preventDefault()} onclick={() => toggleMark("bold")}><b>G</b></button>
        <button class:on={isMarkActive("italic", t)} disabled={family && !family.has_italic} onpointerdown={(e) => e.preventDefault()} onclick={() => toggleMark("italic")}><i>I</i></button>
        <button class:on={isMarkActive("underline", t)} onpointerdown={(e) => e.preventDefault()} onclick={() => toggleMark("underline")}><u>S</u></button>
      </div>
      <div class="row">
        {#each ALIGNS as a (a)}
          <button title={fr.textAlign[a]} aria-label={fr.textAlign[a]} class:on={t.style.align === a} onpointerdown={(e) => e.preventDefault()} onclick={() => setAlign(a)}><AlignIcon align={a} /></button>
        {/each}
      </div>
      <div class="field">
        <span>Couleur</span>
        <ColorPicker value={t.style.color} onchange={setColor} />
      </div>
      {#if selected.zone}
        <div class="field">
          <span>Texte d'invite (visible seulement à l'écran)</span>
          <input value={t.prompt ?? ""} onchange={(e) => updatePrompt(e.currentTarget.value || null)} />
        </div>
      {/if}
    {/if}

    {#if selected.kind.type === "Shape"}
      {@const s = selected.kind}
      {#if !isStrokeOnly(s.shape)}
      <div class="field">
        <span>Remplissage</span>
        <ColorPicker value={s.fill} allowNone onchange={(c) => updateShape({ fill: c })} />
      </div>
      {/if}
      <div class="field">
        <span>Trait : {s.stroke ? `${s.stroke.width_mm} mm` : "aucun"}</span>
        <input
          type="range"
          min={isStrokeOnly(s.shape) ? 0.5 : 0}
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
      <button onclick={() => update((e) => ({ ...e, locked: !e.locked }))}>
        {#if editor.templateMode}📌 {selected.locked ? fr.unfixCell : fr.fixCell}{:else}{selected.locked ? `🔓 ${fr.unlock}` : `🔒 ${fr.lock}`}{/if}
      </button>
      {#if !selected.locked}
        <button onclick={deleteSelection}>🗑 {fr.delete}</button>
      {/if}
    </div>
    {#if editor.templateMode && selected.kind.type !== "Shape"}
      <label class="check">
        <input type="checkbox" checked={selected.zone} onchange={(e) => update((el) => ({ ...el, zone: e.currentTarget.checked }))} />
        {fr.isZone}
      </label>
    {/if}
  {/if}
</aside>
{/if}

<style>
  .right {
    grid-area: right;
    width: 250px;
    background: var(--panel);
    border-left: 1px solid var(--line);
    overflow-y: auto;
    padding: 8px 12px 12px;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  header h3 {
    margin: 0;
  }
  .close {
    border: 0;
    background: transparent;
  }
  .tab {
    grid-area: right;
    align-self: start;
    margin-top: 12px;
    writing-mode: vertical-rl;
    border-radius: 8px 0 0 8px;
    border-right: 0;
    padding: 12px 6px;
    background: var(--panel);
  }
  .frame {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }
  .frame label,
  .rot {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .frame input,
  .rot input {
    width: 100%;
    min-width: 0;
    padding: 3px 6px;
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
