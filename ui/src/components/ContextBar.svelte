<script lang="ts">
  // Barre flottante au-dessus de l'élément sélectionné : seulement ce qui le concerne.
  // Cachée pendant un geste à la souris, un recadrage et en mode modèle.
  import { tick } from "svelte";
  import AlignIcon from "./AlignIcon.svelte";
  import ColorPicker from "./ColorPicker.svelte";
  import GuideLink from "./GuideLink.svelte";
  import StrokeStylePicker from "./StrokeStylePicker.svelte";
  import VAlignIcon from "./VAlignIcon.svelte";
  import FontPicker from "./FontPicker.svelte";
  import { alignFrames, distributeFrames, type AlignMode } from "../lib/align";
  import { placeContextBar, selectionBox } from "../lib/contextBar";
  import { editor } from "../lib/docStore.svelte";
  import { addCaption, deleteSelection, groupSelection, mirrorSelection, rotateSelection, selectionLocked, setFrameImage, toggleLockSelection, ungroupSelection } from "../lib/editActions";
  import { assetUrl } from "../lib/api";
  import { applyStyle, followsStyle, lookOf, newStyleFrom, redefineStyle, stylesOf } from "../lib/namedStyles";
  import { fitCount, repeatBlock } from "../lib/repeat";
  import { endPreview, previewPhoto, previewShape, previewTable, updateTable, previewTextFrame, previewTextStyle, updatePhoto, updateShape, updateTextFrame, updateTextStyle } from "../lib/elementEdit";
  import { textPadding } from "../lib/richtext";
  import { FONT_SIZES, fonts } from "../lib/fonts.svelte";
  import { fr } from "../lib/i18n/fr";
  import { settings } from "../lib/settings.svelte";
  import { handlePadding } from "../lib/geometry";
  import { mmToPx } from "../lib/units";
  import { hasClassField, hasTemplateRole, templateRole, withTemplateRole } from "../lib/templateRole";
  import { fieldsOf, removeField } from "../lib/fieldEdit";
  import { placeField } from "../lib/fieldPlacing.svelte";
  import FieldMenu from "./FieldMenu.svelte";
  import { myImages } from "../lib/myImages.svelte";
  import { isStrokeOnly } from "../lib/shapePaths";
  import { inTableGroup, isTableCell, selectedTable } from "../lib/table";
  import { alignUnits, arrangeUnits, expandSelection, groupOf, isLockedSelection } from "../lib/groups";
  import { blockFrieze, friezeIn, friezeMotifKeys, friezeSettings, removeFrieze, replaceFriezeMotifs, resizeFrieze } from "../lib/decor";
  import FriezePicker from "./FriezePicker.svelte";
  import { cutMarks, type CutStyle } from "../lib/cutMarks";
  import { friezeMotifs, resolveMotifs } from "../lib/friezeMotifs.svelte";
  import { setPictosOutline } from "../lib/pictos.svelte";
  import { ALIGNS, VALIGNS, currentFont, currentSize, isMarkActive, setAlign, setColor, setFont, setSize, setVAlign, toggleMark } from "../lib/textFormat.svelte";
  import type { NamedTextStyle, Frame } from "../lib/types";

  let { pageEl, stageEl }: { pageEl: HTMLElement | undefined; stageEl: HTMLElement | undefined } = $props();

  const ROTATION_HANDLE_GAP = 44;

  type Pop = "corners" | "border" | "color" | "align" | "fill" | "stroke" | "alignMany" | "distribute" | "frame" | "cut" | "fields" | "repeat" | "styles" | "frieze" | "more";
  let pop = $state<Pop | null>(null);
  let barEl = $state<HTMLElement>();
  let pos = $state<{ left: number; top: number } | null>(null);
  /** Largeur au plus de la barre : celle de la zone de travail ; au-delà, ses outils passent à la ligne. */
  let maxWidth = $state<number | null>(null);

  const inGrid = (id: string) => !!editor.page.grid?.cells.some((c) => c.element_id === id);
  // Sélection qui est exactement un groupe : on propose Dissocier ; sinon Grouper (hors cases de grille).
  const isGroup = $derived.by(() => {
    const [first] = editor.selection;
    if (!first || editor.focusedMember) return false;
    const block = groupOf(editor.page, first, editor.inside);
    return block.length > 1 && block.length === editor.selection.length && block.every((id) => editor.selection.includes(id));
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
    if (maxWidth !== Math.floor(s.width - 16)) {
      maxWidth = Math.floor(s.width - 16);
      await tick();
    }
    // La poignée de rotation (Moveable) dépasse au-dessus d'un élément libre sélectionné seul.
    const e = selected;
    const handles = many
      ? !editor.selection.some(inGrid) && !isLockedSelection(editor.page, editor.selection, editor.inside)
      : !!e && !e.locked && !inGrid(e.id) && editor.editingText !== e.id;
    pos = placeContextBar(
      box,
      { width: barEl.offsetWidth, height: barEl.offsetHeight },
      { left: s.left, top: s.top, width: s.width, height: s.height },
      // Un petit élément a ses poignées écartées : la poignée de rotation monte d'autant.
      handles ? ROTATION_HANDLE_GAP + (many || !e ? 0 : handlePadding(mmToPx(e.frame.h_mm, editor.effectiveZoom), settings.bigHandles)) : undefined,
    );
  }

  $effect(() => {
    void [editor.doc, editor.selection, editor.effectiveZoom, editor.drawer, editor.inspectorOpen, editor.editingText, visible, barEl, settings.compactToolbar, settings.moreMenu];
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
  const units = () => alignUnits(editor.page, editor.selection, editor.inside).length;

  function applyFrames(fn: (frames: Frame[]) => Frame[]) {
    const inside = editor.inside;
    editor.updatePage((p) => arrangeUnits(p, editor.selection, fn, inside));
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

  /** Tourner d'un quart de tour : comme les repères de découpe, pas sur une case de grille ou de tableau, ni sur
   *  un bloc verrouillé. */
  const canRotate = $derived(
    editor.selection.length > 0 && !editor.selection.some((id) => inGrid(id) || isTableCell(editor.page, id) || inTableGroup(editor.page, id)) && !isLockedSelection(editor.page, editor.selection, editor.inside),
  );

  /** Miroir : comme la rotation, et seulement s'il y a autre chose qu'un texte (un texte reste lisible). */
  const canMirror = $derived(canRotate && editor.page.elements.some((e) => editor.selection.includes(e.id) && e.kind.type !== "Text" && e.kind.type !== "Table"));

  /** Repères de découpe : pas sur une case de grille ou de tableau, ni sur un bloc verrouillé. */
  const canCut = $derived(
    editor.selection.length > 0 && !editor.selection.some((id) => inGrid(id) || isTableCell(editor.page, id)) && !isLockedSelection(editor.page, editor.selection, editor.inside),
  );

  // Styles nommés : appliquer au texte sélectionné, redéfinir d'après lui, ou en créer un.
  const docStyles = $derived(stylesOf(editor.doc));
  let newStyleName = $state("");

  function useStyle(s: NamedTextStyle) {
    editor.updatePage((p) => ({ ...p, elements: p.elements.map((e) => (editor.selection.includes(e.id) ? applyStyle(e, s) : e)) }));
    pop = null;
  }
  function redefineFromSelected(s: NamedTextStyle) {
    const look = selected && lookOf(selected, s.id, s.name);
    if (look) editor.commit(redefineStyle(editor.doc, look));
    pop = null;
  }
  function createStyle() {
    const made = selected && newStyleFrom(editor.doc, selected, newStyleName);
    if (!made) return;
    editor.commit(made.doc);
    useStyle(made.style);
    newStyleName = "";
  }

  let repeatCols = $state(2);
  let repeatRows = $state(2);

  /** Répète la sélection en grille ; `fill` : autant d'exemplaires que la page en contient. */
  function repeat(fill: boolean) {
    const n = fill ? fitCount(editor.page, expandSelection(editor.page, editor.selection)) : { cols: repeatCols, rows: repeatRows };
    let ids: string[] = [];
    editor.updatePage((p) => {
      const out = repeatBlock(p, editor.selection, n.cols, n.rows);
      ids = out.ids;
      return out.page;
    });
    editor.focusedMember = null;
    editor.insideGroup = [];
    editor.selection = ids;
    pop = null;
  }

  function setCutMarks(style: CutStyle | null) {
    editor.updatePage((p) => cutMarks(p, editor.selection, style));
    editor.focusedMember = null;
    editor.insideGroup = [];
    editor.selection = expandSelection(editor.page, editor.selection.filter((id) => editor.page.elements.some((e) => e.id === id)));
    pop = null;
  }

  // Une frise posée (ou la frise d'un bloc) : sa taille et l'écart entre ses images se règlent après coup.
  const friezeIds = $derived(editor.selection.length > 1 ? friezeIn(editor.page, editor.selection) : null);
  const friezeNow = $derived(friezeIds ? friezeSettings(editor.page, friezeIds) : null);

  function setFrieze(size: number, gap: number, distance?: number) {
    const ids = friezeIds;
    if (!ids) return;
    let fresh: string[] = [];
    editor.updatePage((p) => {
      const r = resizeFrieze(p, ids, size, gap, distance);
      fresh = r.ids;
      return r.page;
    });
    editor.selection = [...editor.selection.filter((id) => !ids.includes(id)), ...fresh];
  }

  // Images d'une frise posée, pendant leur choix dans la fenêtre « Images de la frise ».
  let friezeCodes = $state<string[] | null>(null);

  function editFriezeMotifs() {
    if (!friezeIds) return;
    pop = null;
    friezeCodes = friezeMotifKeys(editor.page, friezeIds);
  }

  /** Les images choisies, dans les cases de la frise : même place, même taille. */
  async function applyFriezeMotifs() {
    const codes = friezeCodes;
    const ids = friezeIds;
    friezeCodes = null;
    if (!codes?.length || !ids || codes.join("|") === friezeMotifKeys(editor.page, ids).join("|")) return;
    const { motifs, assets } = await resolveMotifs(codes, (id) => editor.doc.assets[id] ?? editor.assetMeta(id));
    if (!motifs.length) return;
    let fresh: string[] = [];
    editor.updatePage((p) => {
      const r = replaceFriezeMotifs(p, ids, motifs);
      fresh = r.ids;
      return r.page;
    }, assets);
    editor.selection = [...editor.selection.filter((id) => !ids.includes(id)), ...fresh];
  }

  /** Retire la frise seule : le bloc qu'elle entoure reste. */
  function deleteFrieze() {
    const ids = friezeIds;
    if (!ids) return;
    pop = null;
    editor.updatePage((p) => removeFrieze(p, ids));
    editor.selection = editor.selection.filter((id) => !ids.includes(id));
  }

  /** Un champ : au curseur pendant l'écriture ; sinon un interrupteur, comme les cases cochées du menu : un champ
   *  que le texte contient s'enlève, un autre s'ajoute à la fin. */
  function pickField(key: string) {
    const e = selected;
    if (!e || e.kind.type !== "Text") return;
    if (editor.editingText !== e.id && fieldsOf(e.kind.content).includes(key)) {
      editor.updateElement(e.id, (x) => (x.kind.type === "Text" ? { ...x, kind: { ...x.kind, content: removeField(x.kind.content, key) } } : x));
    } else void placeField(key, e.id);
  }

  function setRole(role: "fill" | "fixed") {
    const id = selected?.id;
    if (id) editor.updateElement(id, (e) => withTemplateRole(e, role));
  }

  async function addBlockFrieze() {
    const id = selected?.id;
    if (!id) return;
    if (friezeMotifs.codes.length === 0) {
      editor.drawer = "decor";
      return;
    }
    const { motifs, assets } = await resolveMotifs(friezeMotifs.codes);
    editor.updatePage((p) => blockFrieze(p, id, motifs) ?? p, assets);
    editor.focusedMember = null;
    editor.insideGroup = [];
    editor.selection = expandSelection(editor.page, [id]);
  }

  function openPhotos() {
    editor.openImages("photos");
  }
</script>

<!-- Frise posée : taille, écart, et ses images à changer. -->
{#snippet friezePop(now: { size: number; gap: number; distance: number; block: boolean })}
  <label>{fr.friezeSize} <input type="range" min="6" max="40" step="1" value={now.size} onchange={(e) => setFrieze(Number(e.currentTarget.value), now.gap)} /> {now.size} mm</label>
  <label>{fr.friezeGap} <input type="range" min="0" max="30" step="1" value={now.gap} onchange={(e) => setFrieze(now.size, Number(e.currentTarget.value))} /> {now.gap} mm</label>
  <!-- Frise de page : sa marge avec le bord ; frise d'un bloc : son écart avec le bloc (0, à cheval sur le bord). -->
  <label title={now.block ? fr.friezeDistanceBlockHint : fr.friezeDistancePageHint}>
    {now.block ? fr.friezeDistanceBlock : fr.friezeDistancePage}
    <input type="range" min="0" max={now.block ? 30 : 60} step="1" value={Math.round(now.distance)} onchange={(e) => setFrieze(now.size, now.gap, Number(e.currentTarget.value))} />
    {Math.round(now.distance)} mm
  </label>
  <button onclick={editFriezeMotifs}>🖼 {fr.friezeChangeImages}</button>
{/snippet}

{#if friezeCodes}<FriezePicker bind:codes={friezeCodes} onclose={applyFriezeMotifs} />{/if}

{#if visible}
  <div
    class="ctx"
    class:compact={settings.compactToolbar}
    role="toolbar"
    tabindex="-1"
    aria-label="Barre de l'élément"
    bind:this={barEl}
    style:max-width={maxWidth ? `${maxWidth}px` : null}
    style:left={pos ? `${pos.left}px` : "-9999px"}
    style:top={pos ? `${pos.top}px` : "-9999px"}
    onpointerdown={keepFocus}
  >
    {#if editor.templateMode && selected && hasTemplateRole(selected)}
      {@const role = templateRole(selected)}
      <!-- Dans un modèle : ce texte ou cette photo est à remplir, ou fixe ; sinon il n'est pas gardé. -->
      <div class="role" role="radiogroup" aria-label={fr.templateRole}>
        <button role="radio" aria-checked={role === "fill"} class:on={role === "fill"} title={fr.templateFillHint} onclick={() => setRole("fill")}>{fr.templateFill}</button>
        <button role="radio" aria-checked={role === "fixed"} class:on={role === "fixed"} title={fr.templateFixedHint} onclick={() => setRole("fixed")}>{fr.templateFixed}</button>
        {#if role === "none"}<span class="dropped">{fr.templateNotKept}</span>{/if}
      </div>
      <span class="sep"></span>
    {:else if editor.templateMode && selected && hasClassField(selected)}
      <span class="dropped">{fr.templateField}</span>
      <span class="sep"></span>
    {/if}
    {#if many}
      {#if isGroup}
        <button class="main" title="Ctrl+Maj+G" onclick={ungroupSelection}>⊟<span class="lbl"> {fr.ungroup}</span></button>
      {:else}
        <button class="main" title="Ctrl+G" disabled={!canGroup} onclick={groupSelection}>⊞<span class="lbl"> {fr.group}</span></button>
      {/if}
      <span class="sep"></span>
      {#if table?.kind.type === "Table"}
        {@const t = table.kind}
        {@const id = table.id}
        <div class="wrap">
          <button class:on={!!t.border} onclick={() => toggle("stroke")}>▦<span class="lbl"> {fr.tableBorders}</span></button>
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
        <button onclick={() => toggle("alignMany")} disabled={units() < 2}>☰<span class="lbl"> {fr.alignMenu} ▾</span></button>
        {#if pop === "alignMany"}
          <div class="pop list">
            {#each alignModes as [mode, label] (mode)}<button onclick={() => applyFrames((f) => alignFrames(f, mode))}>{label}</button>{/each}
          </div>
        {/if}
      </div>
      <div class="wrap">
        <button onclick={() => toggle("distribute")} disabled={units() < 3}>⇹<span class="lbl"> {fr.distribute} ▾</span></button>
        {#if pop === "distribute"}
          <div class="pop list">
            <button onclick={() => applyFrames((f) => distributeFrames(f, "h"))}>{fr.distributeH}</button>
            <button onclick={() => applyFrames((f) => distributeFrames(f, "v"))}>{fr.distributeV}</button>
          </div>
        {/if}
      </div>
      <span class="sep"></span>
      {#if isGroup}
        <!-- Comme l'action : tout le groupe verrouillé, le bouton le déverrouille ; sinon il verrouille. -->
        {@const allLocked = selectionLocked()}
        <button title={allLocked ? fr.unlock : fr.lock} aria-label={allLocked ? fr.unlock : fr.lock} aria-pressed={allLocked} onclick={toggleLockSelection}>{allLocked ? "🔓" : "🔒"}<span class="lbl"> {allLocked ? fr.unlock : fr.lock}</span></button>
      {/if}
      <button onclick={removeSelected}>🗑<span class="lbl"> {fr.delete}</span></button>
    {:else if selected?.kind.type === "Photo"}
      {@const ph = selected.kind}
      {#if !ph.asset}
        <button class="main" onclick={openPhotos}>🖼<span class="lbl"> {fr.choosePhoto}</span></button>
        <button onclick={() => addCaption(selected.id)}>✎<span class="lbl"> {fr.caption}</span></button>
      {:else}
        <button class:on={editor.replacingPhoto} title={fr.replacePhotoHint} onclick={() => ((editor.replacingPhoto = true), editor.openImages("photos"))}>⇄<span class="lbl"> {fr.replace}</span></button>
        <button onclick={() => (editor.cropping = selected.id)}>⛶<span class="lbl"> {fr.crop}</span></button>
        <button data-guide="blur" title={fr.blurHint} onclick={() => (editor.blurring = ph.asset)}>◍<span class="lbl"> {fr.blurButton}</span></button>
        {#if !settings.moreMenu}
        <button title={fr.keepInMyImagesHint} onclick={() => ph.asset && myImages.keep(ph.asset)}>⭐<span class="lbl"> {fr.keepInMyImages}</span></button>
        {/if}
        <div class="wrap">
          <button class:on={pop === "corners"} onclick={() => toggle("corners")}>◜<span class="lbl"> {fr.corners}</span></button>
          {#if pop === "corners"}
            <div class="pop">
              <span>{ph.corner_radius_mm} mm</span>
              <input type="range" min="0" max="20" step="1" value={ph.corner_radius_mm} oninput={(e) => previewPhoto({ corner_radius_mm: Number(e.currentTarget.value) })} onchange={endPreview} />
            </div>
          {/if}
        </div>
        <div class="wrap">
          <button class:on={!!ph.border} onclick={() => toggle("border")}>▢<span class="lbl"> {fr.border}</span></button>
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
        {#if !settings.moreMenu}
        <button class:on={ph.shadow} aria-pressed={ph.shadow} onclick={() => updatePhoto({ shadow: !ph.shadow })}>◩<span class="lbl"> {fr.shadow}</span></button>
        {/if}
        <button onclick={() => addCaption(selected.id)}>✎<span class="lbl"> {fr.caption}</span></button>
        {#if !selected.locked || editor.templateMode}<button title={fr.delete} aria-label={fr.delete} onclick={removeSelected}>🗑</button>{/if}
      {/if}
    {:else if selected?.kind.type === "Text"}
      {@const t = selected.kind}
      {@const family = fonts.families.find((f) => f.name === currentFont(t))}
      {@const named = docStyles.find((s) => s.id === t.style.named)}
      <div class="wrap">
        <button class="style-btn" title={fr.textStylesHint} onclick={() => toggle("styles")}>
          {named ? named.name : fr.textStyleNone}{named && !followsStyle(selected, named) ? " *" : ""} ▾
        </button>
        {#if pop === "styles"}
          <div class="pop styles">
            {#each docStyles as s (s.id)}
              <button class:on={s.id === t.style.named} style:font-family={`"${s.font}"`} style:color={s.color} onclick={() => useStyle(s)}>{s.name}</button>
            {/each}
            {#if named && !followsStyle(selected, named)}
              <button class="update" title={fr.textStyleUpdateHint} onclick={() => redefineFromSelected(named)}>↻<span class="lbl"> {fr.textStyleUpdate(named.name)}</span></button>
            {/if}
            <div class="new-style">
              <input placeholder={fr.textStyleNew} bind:value={newStyleName} onkeydown={(e) => e.key === "Enter" && createStyle()} />
              <button disabled={!newStyleName.trim()} onclick={createStyle}>＋</button>
            </div>
          </div>
        {/if}
      </div>
      <FontPicker value={currentFont(t)} onpick={setFont} compact />
      <select title={fr.size} value={String(currentSize(t))} onchange={(e) => setSize(Number(e.currentTarget.value))}>
        {#each FONT_SIZES as s (s)}<option value={String(s)}>{s}</option>{/each}
      </select>
      <span class="sep"></span>
      <button title={fr.bold} class:on={isMarkActive("bold", t)} disabled={family && !family.has_bold} onclick={() => toggleMark("bold")}><b>G</b></button>
      <button title={fr.italic} class:on={isMarkActive("italic", t)} disabled={family && !family.has_italic} onclick={() => toggleMark("italic")}><i>I</i></button>
      <button title={fr.underline} class:on={isMarkActive("underline", t)} onclick={() => toggleMark("underline")}><u>S</u></button>
      <div class="wrap">
        <button class="field-btn" title={fr.fieldButtonHint} onclick={() => toggle("fields")}>&#123; &#125;<span class="lbl"> {fr.fieldButton}</span> ▾</button>
        {#if pop === "fields"}
          <div class="pop fields">
            <FieldMenu current={editor.editingText === selected?.id ? [] : fieldsOf(t.content)} onpick={(key) => (pickField(key), (pop = null))} />
          </div>
        {/if}
      </div>
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
        <button class:on={!!t.frame} onclick={() => toggle("frame")}>▭<span class="lbl"> {fr.textFrame}</span></button>
        {#if pop === "frame"}
          {@const f = t.frame}
          <div class="pop frame">
            <span>{fr.frameFill}</span>
            <ColorPicker value={f?.fill ?? null} allowNone onchange={(c) => updateTextFrame({ fill: c })} />
            <span>{fr.writingLines}</span>
            <div class="patterns">
              {#each [null, "Seyes", "Lines", "SmallSquares", "Dots"] as const as pat (pat)}
                <button class:on={(f?.pattern ?? null) === pat} onclick={() => updateTextFrame({ pattern: pat })}>{fr.patternLabel[pat ?? "None"]}</button>
              {/each}
            </div>
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
      <button class:on={editor.replacingPictos} onclick={() => ((editor.replacingPictos = true), editor.openImages("pictos"))}>⇄<span class="lbl"> {fr.replace}</span></button>
      {@const colourIn = selected.kind.outline === true}
      <button class:on={colourIn} aria-pressed={colourIn} title={fr.pictoOutlineHint} onclick={() => editor.updatePage((p) => setPictosOutline(p, editor.selection, !colourIn))}>✎<span class="lbl"> {fr.pictoOutlineShort}</span></button>
      {#if !selected.locked || editor.templateMode}<button title={fr.delete} aria-label={fr.delete} onclick={removeSelected}>🗑</button>{/if}
    {:else if selected?.kind.type === "Shape"}
      {@const s = selected.kind}
      {#if !isStrokeOnly(s.shape) && s.shape !== "Path"}
      <div class="wrap">
        <button onclick={() => toggle("fill")}><span class="dot" style:background={s.fill ?? "transparent"}></span><span class="lbl"> {fr.fill}</span></button>
        {#if pop === "fill"}<div class="pop"><ColorPicker value={s.fill} allowNone onchange={(c) => updateShape({ fill: c })} /></div>{/if}
      </div>
      {/if}
      <div class="wrap">
        <button class:on={!!s.stroke} onclick={() => toggle("stroke")}>▢<span class="lbl"> {fr.stroke}</span></button>
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
          <button onclick={() => toggle("corners")}>◜<span class="lbl"> {fr.corners}</span></button>
          {#if pop === "corners"}
            <div class="pop">
              <span>{s.corner_radius_mm} mm</span>
              <input type="range" min="0" max="20" step="1" value={s.corner_radius_mm} oninput={(e) => previewShape({ corner_radius_mm: Number(e.currentTarget.value) })} onchange={endPreview} />
            </div>
          {/if}
        </div>
      {/if}
      {#if !selected.locked || editor.templateMode}<button title={fr.delete} aria-label={fr.delete} onclick={removeSelected}>🗑</button>{/if}
    {/if}
    {#if settings.moreMenu}
    <!-- Les outils de tous les jours sont dans la barre ; les autres dans « Plus », où s'ouvrent aussi leurs
           réglages (taille de la frise, répéter, repères de découpe). -->
      <span class="sep"></span>
      <div class="wrap">
        <button class:on={pop === "more" || pop === "frieze" || pop === "repeat" || pop === "cut"} aria-haspopup="menu" title={fr.moreTools} onclick={() => toggle("more")}>⋯<span class="lbl"> {fr.moreTools} ▾</span></button>
        {#if pop === "more"}
          {@const ph = selected?.kind.type === "Photo" && selected.kind.asset ? selected.kind : null}
          <div class="pop list more" role="menu">
            {#if ph}
              <button role="menuitem" class:on={ph.shadow} onclick={() => updatePhoto({ shadow: !ph.shadow })}>◩ {fr.shadow}</button>
              <button role="menuitem" title={fr.keepInMyImagesHint} onclick={() => ((pop = null), ph.asset && myImages.keep(ph.asset))}>⭐ {fr.keepInMyImages}</button>
            {/if}
            {#if canMirror}
              <button role="menuitem" title={fr.mirrorHHint} onclick={() => mirrorSelection("h")}>⇋ {fr.mirrorH}</button>
              <button role="menuitem" title={fr.mirrorVHint} onclick={() => mirrorSelection("v")}>⇅ {fr.mirrorV}</button>
            {/if}
            {#if canRotate}
              <button role="menuitem" onclick={() => rotateSelection(-90)}>↺ {fr.rotateLeftShort}</button>
              <button role="menuitem" onclick={() => rotateSelection(90)}>↻ {fr.rotateRightShort}</button>
            {/if}
            {#if canFrieze}
              <button role="menuitem" data-guide="frieze-around" title={fr.blockFriezeHint} onclick={() => ((pop = null), addBlockFrieze())}>❀ {fr.blockFriezeAround}</button>
            {/if}
            {#if friezeIds && friezeNow}
              <button role="menuitem" data-guide="frieze-size" title={fr.friezeEditHint2} onclick={() => (pop = "frieze")}>❀ {fr.friezeEdit} ▸</button>
              <button role="menuitem" class="danger" onclick={deleteFrieze}>🗑 {fr.friezeDelete}</button>
            {/if}
            {#if canCut}
              <button role="menuitem" data-guide="repeat" title={fr.repeatHint} onclick={() => (pop = "repeat")}>▦ {fr.repeat} ▸</button>
              <button role="menuitem" data-guide="cut" title={fr.cutMarksHint} onclick={() => (pop = "cut")}>✂ {fr.cutMarks} ▸</button>
            {/if}
            {#if !many}
              <button role="menuitem" onclick={() => ((pop = null), (editor.inspectorOpen = true))}>⚙ {fr.allSettings}</button>
            {/if}
          </div>
        {:else if pop === "frieze" && friezeNow}
          <div class="pop frieze">{@render friezePop(friezeNow)}</div>
        {:else if pop === "repeat"}
          <div class="pop repeat">
            <label>{fr.tableCols} <input type="number" min="1" max="12" bind:value={repeatCols} /></label>
            <label>{fr.tableRows} <input type="number" min="1" max="20" bind:value={repeatRows} /></label>
            <button class="primary" disabled={repeatCols * repeatRows < 2} onclick={() => repeat(false)}>{fr.repeatApply(repeatCols * repeatRows)}</button>
            <button onclick={() => repeat(true)}>{fr.repeatFill}</button>
            <GuideLink id="etiquettes" />
          </div>
        {:else if pop === "cut"}
          <div class="pop list">
            <button onclick={() => setCutMarks("corners")}>⌜ ⌝ {fr.cutCorners}</button>
            <button onclick={() => setCutMarks("dashed")}>✂ ┄ {fr.cutDashed}</button>
            <button onclick={() => setCutMarks(null)}>{fr.cutNone}</button>
          </div>
        {/if}
      </div>
    {:else}
    {#if canMirror}
        {@const mh = selected ? (selected.kind.type === "Photo" ? !!selected.kind.mirrored : !!selected.flip_h) : false}
        <!-- Un seul outil, deux sens : gauche-droite et haut-bas côte à côte, sans menu. -->
        <div class="mirror" role="group" aria-label={fr.mirrorRow}>
          <span class="mirror-name">{fr.mirrorRow}</span>
          <button class:on={mh} aria-pressed={mh} title={fr.mirrorHHint} aria-label={fr.mirrorH} onclick={() => mirrorSelection("h")}>⇋<span class="lbl">{fr.mirrorH}</span></button>
          <button class:on={!!selected?.flip_v} aria-pressed={!!selected?.flip_v} title={fr.mirrorVHint} aria-label={fr.mirrorV} onclick={() => mirrorSelection("v")}>⇅<span class="lbl">{fr.mirrorV}</span></button>
        </div>
      {/if}
      {#if canRotate}
        <button title={fr.rotateLeft} aria-label={fr.rotateLeft} onclick={() => rotateSelection(-90)}>↺</button>
        <button title={fr.rotateRight} aria-label={fr.rotateRight} onclick={() => rotateSelection(90)}>↻</button>
      {/if}
      {#if canFrieze}
        <button data-guide="frieze-around" title={fr.blockFriezeHint} onclick={addBlockFrieze}>❀<span class="lbl"> {fr.blockFriezeAround}</span></button>
      {/if}
      {#if friezeIds && friezeNow}
        <div class="wrap">
          <button data-guide="frieze-size" class:on={pop === "frieze"} title={fr.friezeEditHint2} onclick={() => toggle("frieze")}>❀<span class="lbl"> {fr.friezeEdit} ▾</span></button>
          {#if pop === "frieze"}
            <div class="pop frieze">{@render friezePop(friezeNow)}</div>
          {/if}
        </div>
        <button class="danger" title={fr.friezeDeleteHint} onclick={deleteFrieze}>🗑<span class="lbl"> {fr.friezeDelete}</span></button>
      {/if}
      {#if canCut}
        <div class="wrap">
          <button data-guide="repeat" title={fr.repeatHint} onclick={() => toggle("repeat")}>▦<span class="lbl"> {fr.repeat} ▾</span></button>
          {#if pop === "repeat"}
            <div class="pop repeat">
              <label>{fr.tableCols} <input type="number" min="1" max="12" bind:value={repeatCols} /></label>
              <label>{fr.tableRows} <input type="number" min="1" max="20" bind:value={repeatRows} /></label>
              <button class="primary" disabled={repeatCols * repeatRows < 2} onclick={() => repeat(false)}>{fr.repeatApply(repeatCols * repeatRows)}</button>
              <button onclick={() => repeat(true)}>{fr.repeatFill}</button>
              <GuideLink id="etiquettes" />
            </div>
          {/if}
        </div>
        <div class="wrap">
          <button data-guide="cut" title={fr.cutMarksHint} onclick={() => toggle("cut")}>✂<span class="lbl"> {fr.cutMarks} ▾</span></button>
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
        <button title={fr.allSettings} aria-label={fr.allSettings} onclick={() => (editor.inspectorOpen = true)}>⋯</button>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .pop.styles {
    display: grid;
    gap: 4px;
    min-width: 220px;
  }
  .pop.styles > button {
    text-align: left;
    font-size: 16px;
  }
  .pop.styles .update {
    font-family: inherit !important;
    color: var(--accent-strong) !important;
    font-size: 13px;
  }
  .new-style {
    display: flex;
    gap: 4px;
  }
  .new-style input {
    flex: 1;
  }
  /* À remplir / Fixe : toujours écrit en entier, c'est le choix qui compte le plus dans un modèle. */
  .role {
    display: inline-flex;
    align-items: center;
    border: 1px solid var(--line);
    border-radius: 7px;
    overflow: hidden;
  }
  .role button {
    border: 0;
    border-radius: 0;
    background: white;
    padding: 4px 9px;
  }
  .role button + button {
    border-left: 1px solid var(--line);
  }
  .dropped {
    padding: 0 8px;
    font-size: 12px;
    color: #b35c00;
  }
  /* Miroir : le nom, puis les deux sens, réunis dans un même cadre. */
  .mirror {
    display: inline-flex;
    align-items: center;
    border: 1px solid var(--line);
    border-radius: 7px;
  }
  .mirror button:last-child {
    border-radius: 0 6px 6px 0;
  }
  .compact .mirror button:first-of-type {
    border-radius: 6px 0 0 6px;
  }
  .mirror-name {
    padding: 0 6px 0 8px;
  }
  .compact .mirror-name,
  .ctx:not(.compact) .mirror :global(.lbl) {
    display: none;
  }
  .mirror button {
    border: 0;
    border-radius: 0;
    background: transparent;
    padding: 4px 8px;
  }
  .mirror button + button {
    border-left: 1px solid var(--line);
  }
  .mirror button:hover {
    background: #f3efe8;
  }
  /* « Plus » est au bout de la barre : son menu s'aligne sur son bord droit, une ligne par outil. */
  .pop.more {
    left: auto;
    right: 0;
  }
  .pop.more > button {
    white-space: nowrap;
    text-align: left;
  }
  .pop.frieze {
    min-width: 260px;
  }
  .pop.frieze > button {
    justify-self: start;
  }
  .danger:not(:disabled):hover {
    color: #c0392b;
    background: #fdecea;
  }
  .pop.frieze label {
    display: grid;
    grid-template-columns: 6em 1fr 3.5em;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }
  .pop.repeat {
    display: grid;
    gap: 6px;
    min-width: 200px;
  }
  .pop.repeat label {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }
  .pop.repeat input {
    width: 60px;
  }
  .patterns {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .pop.fields {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    min-width: 200px;
  }
  .pop.fields button {
    text-align: left;
  }
  .hollow-a {
    -webkit-text-fill-color: transparent;
    -webkit-text-stroke: 1px currentColor;
    font-weight: 700;
  }
  .ctx {
    position: fixed;
    display: flex;
    /* Trop d'outils pour la largeur de l'écran : ils passent à la ligne, aucun n'est caché. */
    flex-wrap: wrap;
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
  /* Icône et nom écrits par défaut. Avec la préférence « icônes seules » (.compact), seul le nom des outils
     disparaît (pas celui de l'action principale, .main) : au survol ou au clavier, il apparaît dans une
     languette de la barre, au-dessus de l'icône, de la même couleur et collée à son bord, sans rien déplacer. */
  .compact > button:not(.main),
  .compact > .wrap > button:not(.main),
  .compact .mirror > button {
    position: relative;
  }
  .compact > button:not(.main) > :global(.lbl),
  .compact > .wrap > button:not(.main) > :global(.lbl),
  .compact .mirror > button > :global(.lbl) {
    position: absolute;
    /* Le bas de la languette recouvre le bord haut de la barre (marge de 3 px, bord de 1 px) : elle en sort. */
    bottom: calc(100% + 2px);
    left: 50%;
    transform: translateX(-50%);
    padding: 4px 10px 3px;
    border: 1px solid var(--line);
    border-bottom: 0;
    border-radius: 8px 8px 0 0;
    background: white;
    color: inherit;
    font-size: 13px;
    white-space: nowrap;
    pointer-events: none;
    box-shadow: 0 -3px 8px rgba(0, 0, 0, 0.08);
    /* Cachée, puis montrée après l'attente choisie dans les Préférences (aucune par défaut) ; cachée aussitôt. */
    visibility: hidden;
  }
  .compact > button:not(.main):hover > :global(.lbl),
  .compact > button:not(.main):focus-visible > :global(.lbl),
  .compact .mirror > button:hover > :global(.lbl),
  .compact .mirror > button:focus-visible > :global(.lbl),
  .compact > .wrap > button:not(.main):hover > :global(.lbl),
  .compact > .wrap > button:not(.main):focus-visible > :global(.lbl) {
    visibility: visible;
    transition: visibility 0s var(--label-delay, 0s);
  }
  /* L'espace entre l'icône et le nom écrit (Svelte retire l'espace au début du nom). */
  .ctx:not(.compact) :global(.lbl)::before,
  .ctx .main :global(.lbl)::before {
    content: "\00a0";
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
