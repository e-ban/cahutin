<script lang="ts">
  import { isPlacedCell } from "../../lib/table";
  import AlignIcon from "../AlignIcon.svelte";
  import VAlignIcon from "../VAlignIcon.svelte";
  import ColorPicker from "../ColorPicker.svelte";
  import StrokeStylePicker from "../StrokeStylePicker.svelte";
  import FontPicker from "../FontPicker.svelte";
  import Icon from "../Icon.svelte";
  import Section from "./InspectorSection.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { hasTemplateRole, templateRole, withTemplateRole } from "../../lib/templateRole";
  import { isStrokeOnly } from "../../lib/shapePaths";
  import { FONT_SIZES, fonts } from "../../lib/fonts.svelte";
  import { ALIGNS, VALIGNS, activeTip, currentFont, currentSize, isMarkActive, setAlign, setColor, setFont, setSize, setVAlign, toggleMark, updatePrompt } from "../../lib/textFormat.svelte";
  import { endPreview, previewPhoto, previewShape, previewTextStyle, updatePhoto, updateSelected, updateShape } from "../../lib/elementEdit";
  import { cropPosition, photoTrims, roundedFrame, wholePhoto, withPosition, withTrims, type Trims } from "../../lib/crop";
  import { bringSelection, deleteSelection, mirrorSelection, rotateSelection, selectionLocked, setSelectedFrame, setSelectionSize, toggleLockSelection, ungroupSelection } from "../../lib/editActions";
  import { boundsOf, isLockedSelection } from "../../lib/groups";
  import type { AssetMeta, Crop, Element, Frame } from "../../lib/types";
  import { textPadding } from "../../lib/richtext";

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

  // Position et taille exactes : une case de grille se règle en mode modèle, une case de tableau suit son
  // tableau, un élément verrouillé ne bouge pas.
  const frameFixed = $derived(!!selected && (selected.locked || isPlacedCell(editor.page, selected.id)));

  function setFrame(key: keyof Frame, raw: string) {
    setSelectedFrame(key, Number(raw.replace(",", ".")), keepRatio);
  }

  // Garder les proportions : coché d'office pour une photo ou une image, à chaque nouvel élément choisi.
  let keepRatio = $state(true);
  let ratioFor: string | null = null;
  $effect.pre(() => {
    const s = selected;
    if (s && s.id !== ratioFor) {
      ratioFor = s.id;
      keepRatio = s.kind.type === "Photo" || s.kind.type === "Image";
    }
  });

  /** Un texte ou un tableau ne se retourne pas (il resterait illisible). */
  const mirrorable = $derived(!!selected && selected.kind.type !== "Text" && selected.kind.type !== "Table");
  const num = (raw: string) => Number(raw.replace(",", "."));

  // Recadrage d'une photo : rognage par bord, agrandissement et position dans le cadre.
  const photoMeta = $derived(selected?.kind.type === "Photo" && selected.kind.asset ? (editor.doc.assets[selected.kind.asset] ?? editor.assetMeta(selected.kind.asset)) : undefined);
  /** Photo retournée à l'écran : miroir de la photo et retournement de l'élément se combinent. */
  const flips = (e: Element) => [e.kind.type === "Photo" && !!e.kind.mirrored !== !!e.flip_h, !!e.flip_v] as const;
  const round1 = (v: number) => Math.round(v * 10) / 10;

  /** Une photo rognée, son cadre ou son recadrage changé par `fn`. */
  function reframe(fn: (meta: AssetMeta, e: Element, crop: Crop) => { frame: Frame; crop: Crop }) {
    const meta = photoMeta;
    if (!meta) return;
    update((e) => {
      if (e.kind.type !== "Photo") return e;
      const r = fn(meta, e, e.kind.crop);
      return { ...e, frame: roundedFrame(r.frame), kind: { ...e.kind, crop: r.crop } };
    });
  }

  function trimSide(side: keyof Trims, raw: string) {
    const v = num(raw);
    if (!Number.isFinite(v)) return;
    reframe((meta, e, crop) => withTrims(meta, e.frame, crop, { ...photoTrims(meta, e.frame, crop, ...flips(e)), [side]: v }, ...flips(e)));
  }

  function previewCrop(fn: (meta: AssetMeta, e: Element, crop: Crop) => Crop) {
    const meta = photoMeta;
    const e = selected;
    if (!meta || !e || e.kind.type !== "Photo") return;
    previewPhoto({ crop: fn(meta, e, e.kind.crop) });
  }

</script>

<!-- Les mêmes réglages pour tout : quart de tour, miroir (sauf un texte), devant ou derrière. -->
{#snippet quarterTurns(fixed: boolean)}
  <button disabled={fixed} title={fr.rotateLeft} onclick={() => rotateSelection(-90)}><Icon name="rotateLeft" /> 90°</button>
  <button disabled={fixed} title={fr.rotateRight} onclick={() => rotateSelection(90)}><Icon name="rotateRight" /> 90°</button>
{/snippet}

{#snippet turnAndOrder(fixed: boolean, mirror: boolean)}
  {#if mirror}
    <div class="field">
      <span>{fr.mirrorRow}</span>
      <div class="row">
        <button disabled={fixed} title={fr.mirrorHHint} onclick={() => mirrorSelection("h")}><Icon name="flipH" /> {fr.mirrorH}</button>
        <button disabled={fixed} title={fr.mirrorVHint} onclick={() => mirrorSelection("v")}><Icon name="flipV" /> {fr.mirrorV}</button>
      </div>
    </div>
  {/if}
  <div class="field">
    <span>{fr.orderRow}</span>
    <div class="row">
      <button onclick={() => bringSelection("front")}><Icon name="bringFront" /> {fr.bringFront}</button>
      <button onclick={() => bringSelection("back")}><Icon name="sendBack" /> {fr.sendBack}</button>
    </div>
  </div>
{/snippet}

{#if !editor.inspectorOpen}
  <button class="tab" title={fr.openInspector} aria-label={fr.openInspector} onclick={() => (editor.inspectorOpen = true)}><Icon name="settings" /> {fr.inspector} <Icon name="chevronLeft" size={14} /></button>
{:else}
<aside class="right" aria-label={fr.inspector}>
  <header>
    <h3>{fr.inspector}</h3>
    <button class="close" title={fr.closeInspector} aria-label={fr.closeInspector} onclick={() => (editor.inspectorOpen = false)}><Icon name="chevronRight" /></button>
  </header>
  {#if groupIds}
    <h3>{fr.groupOf(groupIds.length)}</h3>
    <Section id="frame" title={fr.sectionFrame}>
    <div class="field">
      <span>{fr.groupPosition}</span>
      <div class="frame">
        <label>{fr.posX}<input type="number" step="0.5" disabled={groupLocked} value={Math.round(groupBox!.x_mm * 10) / 10} onchange={(e) => moveGroup("x", e.currentTarget.value)} /></label>
        <label>{fr.posY}<input type="number" step="0.5" disabled={groupLocked} value={Math.round(groupBox!.y_mm * 10) / 10} onchange={(e) => moveGroup("y", e.currentTarget.value)} /></label>
      </div>
    </div>
    <div class="field">
      <span>{fr.sizeMm}</span>
      <div class="frame">
        <label>{fr.width}<input type="number" step="0.5" min="1" disabled={groupLocked} value={Math.round(groupBox!.w_mm * 10) / 10} onchange={(e) => setSelectionSize("w_mm", num(e.currentTarget.value))} /></label>
        <label>{fr.height}<input type="number" step="0.5" min="1" disabled={groupLocked} value={Math.round(groupBox!.h_mm * 10) / 10} onchange={(e) => setSelectionSize("h_mm", num(e.currentTarget.value))} /></label>
      </div>
      <small class="hint">{fr.groupKeepsRatio}</small>
    </div>
    <div class="field">
      <span>{fr.rotationRow}</span>
      <div class="row">{@render quarterTurns(groupLocked)}</div>
    </div>
    </Section>
    <Section id="arrange" title={fr.sectionArrange} open={false}>{@render turnAndOrder(groupLocked, true)}</Section>
    <div class="actions">
      <button onclick={ungroupSelection}><Icon name="ungroup" /> {fr.ungroup}</button>
      <!-- Même règle que la barre : tout le groupe verrouillé, on le déverrouille. -->
      <button onclick={toggleLockSelection}>{#if selectionLocked()}<Icon name="unlock" /> {fr.unlock}{:else}<Icon name="lock" /> {fr.lock}{/if}</button>
      <button onclick={deleteSelection}><Icon name="trash" /> {fr.delete}</button>
    </div>
  {:else if !selected}
    <p class="hint">{fr.noSelection}</p>
  {:else}
    <h3>{selected.kind.type === "Photo" ? fr.photo : selected.kind.type === "Text" ? fr.text : selected.kind.type === "Image" ? fr.picture : fr.shape}</h3>

    <Section id="frame" title={fr.sectionFrame}>
    <div class="field">
      <span>{fr.positionOnly}</span>
      <div class="frame">
        {#each [["x_mm", fr.posX], ["y_mm", fr.posY]] as [key, label] (key)}
          <label>{label}<input type="number" step="0.5" disabled={frameFixed} value={selected.frame[key as keyof Frame]} onchange={(e) => setFrame(key as keyof Frame, e.currentTarget.value)} /></label>
        {/each}
      </div>
    </div>
    <div class="field">
      <span>{fr.sizeMm}</span>
      <div class="frame">
        {#each [["w_mm", fr.width], ["h_mm", fr.height]] as [key, label] (key)}
          <label>{label}<input type="number" step="0.5" min="1" disabled={frameFixed} value={selected.frame[key as keyof Frame]} onchange={(e) => setFrame(key as keyof Frame, e.currentTarget.value)} /></label>
        {/each}
      </div>
      {#if selected.kind.type !== "Image"}
        <label class="check"><input type="checkbox" disabled={frameFixed} bind:checked={keepRatio} /> {fr.keepRatio}</label>
      {/if}
      {#if frameFixed && !selected.locked}<small class="hint">{fr.gridLockedFrame}</small>{/if}
    </div>
    <div class="field">
      <span>{fr.rotation}</span>
      <div class="row">
        <input class="deg" type="number" step="1" disabled={frameFixed} value={selected.frame.rotation_deg} onchange={(e) => setFrame("rotation_deg", e.currentTarget.value)} />
        {@render quarterTurns(frameFixed)}
      </div>
    </div>
    </Section>
    <Section id="arrange" title={fr.sectionArrange} open={false}>{@render turnAndOrder(frameFixed, mirrorable)}</Section>

    {#if selected.kind.type === "Photo"}
      {@const ph = selected.kind}
      {#if ph.asset && photoMeta}
        {@const meta = photoMeta}
        {@const trims = photoTrims(meta, selected.frame, ph.crop, ...flips(selected))}
        {@const pos = cropPosition(meta, selected.frame, ph.crop, ...flips(selected))}
        {@const cropFixed = selected.locked && !editor.templateMode}
        <Section id="crop" title={fr.cropSection} open={false}>
        <div class="field crop">
          <p class="hint">{fr.cropHint}</p>
          <span>{fr.trimRow}</span>
          <div class="frame trims">
            {#each [["top", fr.trimTop], ["bottom", fr.trimBottom], ["left", fr.trimLeft], ["right", fr.trimRight]] as [side, label] (side)}
              <label>{label}<input type="number" step="0.5" min="0" disabled={frameFixed} value={round1(trims[side as keyof Trims])} onchange={(e) => trimSide(side as keyof Trims, e.currentTarget.value)} /></label>
            {/each}
          </div>
          <span>{fr.cropZoom(Math.round(ph.crop.scale * 100))}</span>
          <input type="range" min="100" max={Math.max(500, Math.ceil(ph.crop.scale * 100))} step="5" disabled={cropFixed} value={Math.round(ph.crop.scale * 100)} oninput={(e) => previewCrop((_, __, crop) => ({ ...crop, scale: Number(e.currentTarget.value) / 100 }))} onchange={endPreview} />
          {#each [["x", fr.cropPosX], ["y", fr.cropPosY]] as [axis, label] (axis)}
            {@const p = pos[axis as "x" | "y"]}
            <!-- Sa valeur après le nom, comme pour les autres curseurs (aucune quand rien n'est caché). -->
            <span>{label}{p === null ? "" : ` : ${Math.round(p * 100)} %`}</span>
            <input
              type="range"
              min="0"
              max="100"
              step="1"
              disabled={cropFixed || p === null}
              title={p === null ? fr.cropNothingHidden : undefined}
              value={p === null ? 50 : Math.round(p * 100)}
              oninput={(e) => previewCrop((m, el, crop) => withPosition(m, el.frame, crop, { [axis]: Number(e.currentTarget.value) / 100 }, ...flips(el)))}
              onchange={endPreview}
            />
          {/each}
          <button disabled={frameFixed} title={fr.wholePhotoHint} onclick={() => reframe((m, e, crop) => wholePhoto(m, e.frame, crop, ...flips(e), editor.page.size.width_mm, editor.page.size.height_mm))}><Icon name="photo" /> {fr.wholePhoto}</button>
        </div>
        </Section>
      {/if}
      <Section id="look" title={fr.sectionLook}>
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
            oninput={(e) => previewPhoto({ border: { ...ph.border!, width_mm: Number(e.currentTarget.value) } })}
            onchange={endPreview}
          />
          <ColorPicker value={ph.border.color} onchange={(c) => updatePhoto({ border: { ...ph.border!, color: c ?? "#ffffff" } })} />
          <StrokeStylePicker value={ph.border.style} onpick={(style) => updatePhoto({ border: { ...ph.border!, style: style === "Solid" ? null : style } })} />
        {/if}
      </div>
      <div class="field">
        <label class="check">
          <input type="checkbox" checked={ph.shadow} onchange={(e) => updatePhoto({ shadow: e.currentTarget.checked })} />
          Ombre
        </label>
      </div>
      </Section>
      {#if ph.asset}
        <button onclick={() => updatePhoto({ asset: null, crop: { scale: 1, center_x: 0.5, center_y: 0.5 } })}>Retirer la photo</button>
      {/if}
    {/if}

    {#if selected.kind.type === "Text"}
      {@const t = selected.kind}
      {@const family = fonts.families.find((f) => f.name === currentFont(t))}
      <Section id="text" title={fr.sectionFormat}>
      {#if !tip}
        <p class="hint">Double-cliquez pour écrire. Sans édition, les réglages s'appliquent à toute la zone.</p>
      {/if}
      <div class="field">
        <span>Police</span>
        <FontPicker value={currentFont(t)} onpick={setFont} />
      </div>
      <div class="field">
        <span>Taille</span>
        <select value={String(currentSize(t))} onchange={(e) => setSize(Number(e.currentTarget.value))}>
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
      <div class="row">
        {#each VALIGNS as v (v)}
          <button title={fr.textVAlign[v]} aria-label={fr.textVAlign[v]} class:on={(t.style.valign ?? "Top") === v} onpointerdown={(e) => e.preventDefault()} onclick={() => setVAlign(v)}><VAlignIcon valign={v} /></button>
        {/each}
      </div>
      <div class="field">
        <span>{fr.padding} : {textPadding(t.style)} mm</span>
        <input type="range" min="0" max="10" step="0.5" value={textPadding(t.style)} oninput={(e) => previewTextStyle({ padding_mm: Number(e.currentTarget.value) })} onchange={endPreview} />
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
      </Section>
    {/if}

    {#if selected.kind.type === "Shape"}
      {@const s = selected.kind}
      <Section id="look" title={fr.sectionLook}>
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
            previewShape({ stroke: w === 0 ? null : { width_mm: w, color: s.stroke?.color ?? "#999999", style: s.stroke?.style ?? null } });
          }}
          onchange={endPreview}
        />
        {#if s.stroke}
          <ColorPicker value={s.stroke.color} onchange={(c) => updateShape({ stroke: { ...s.stroke!, color: c ?? "#999999" } })} />
          <StrokeStylePicker value={s.stroke.style} onpick={(style) => updateShape({ stroke: { ...s.stroke!, style: style === "Solid" ? null : style } })} />
        {/if}
      </div>
      {#if s.shape === "Rect"}
        <div class="field">
          <span>Coins arrondis : {s.corner_radius_mm} mm</span>
          <input type="range" min="0" max="20" step="1" value={s.corner_radius_mm} oninput={(e) => previewShape({ corner_radius_mm: Number(e.currentTarget.value) })} onchange={endPreview} />
        </div>
      {/if}
      </Section>
    {/if}

    {#if editor.templateMode && hasTemplateRole(selected)}
      {@const role = templateRole(selected)}
      <div class="role" role="radiogroup" aria-label={fr.templateRole}>
        <span>{fr.templateRole}</span>
        <button role="radio" aria-checked={role === "fill"} class:on={role === "fill"} title={fr.templateFillHint} onclick={() => update((e) => withTemplateRole(e, "fill"))}>{fr.templateFill}</button>
        <button role="radio" aria-checked={role === "fixed"} class:on={role === "fixed"} title={fr.templateFixedHint} onclick={() => update((e) => withTemplateRole(e, "fixed"))}>{fr.templateFixed}</button>
      </div>
      {#if role === "none"}<p class="dropped">{fr.templateNotKept}</p>{/if}
    {/if}
    <div class="actions">
      {#if !(editor.templateMode && hasTemplateRole(selected))}
      <button onclick={() => update((e) => ({ ...e, locked: !e.locked }))}>
        {#if editor.templateMode}<Icon name="pin" /> {selected.locked ? fr.unfixCell : fr.fixCell}{:else if selected.locked}<Icon name="unlock" /> {fr.unlock}{:else}<Icon name="lock" /> {fr.lock}{/if}
      </button>
      {/if}
      {#if !selected.locked}
        <button onclick={deleteSelection}><Icon name="trash" /> {fr.delete}</button>
      {/if}
    </div>

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
  /* Titres calmes : écrits normalement, en gras léger, de la couleur du texte. */
  h3 {
    font-size: 15px;
    font-weight: 600;
    text-transform: none;
    letter-spacing: normal;
    color: var(--text);
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
  .frame label {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .frame input {
    width: 100%;
    min-width: 0;
    padding: 3px 6px;
  }
  /* Rognage : « À gauche », « En haut » au-dessus de leur case, sur une ligne. */
  .trims input {
    box-sizing: border-box;
  }
  .trims label {
    flex-direction: column;
    align-items: stretch;
    gap: 2px;
    white-space: nowrap;
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
  .role {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
  }
  .role span {
    flex: 1;
  }
  .role .on {
    background: #e8f1fb;
    border-color: var(--accent);
    color: var(--accent);
  }
  .dropped {
    margin: 0;
    font-size: 12px;
    color: #b35c00;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .row button {
    flex: 1;
    white-space: nowrap;
  }
  .deg {
    width: 4.5em;
    flex: none;
  }
  .actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    margin-top: 18px;
  }
</style>
