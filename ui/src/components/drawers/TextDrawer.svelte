<script lang="ts">
  import Drawer from "../layout/Drawer.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { tableAdd } from "../../lib/api";
  import { centeredFrame, textElement } from "../../lib/factory";
  import { fr } from "../../lib/i18n/fr";
  import { withTemplateRole } from "../../lib/templateRole";
  import { fieldText } from "../../lib/fieldEdit";
  import { insertField } from "../../lib/textFormat.svelte";
  import FieldMenu from "../FieldMenu.svelte";
  import InspectorSection from "../layout/InspectorSection.svelte";
  import type { Align, Element, Pattern, RichText } from "../../lib/types";

  interface Preset {
    label: string;
    text: string;
    font: string;
    size_pt: number;
    align: Align;
    italic: boolean;
    h_mm: number;
    color?: string;
    outline?: boolean;
  }

  const presets: Preset[] = [
    { label: fr.presetTitle, text: fr.presetTitleText, font: "Playwrite FR Moderne", size_pt: 24, align: "Center", italic: false, h_mm: 22 },
    { label: fr.presetParagraph, text: fr.presetParagraphText, font: "Andika", size_pt: 14, align: "Left", italic: false, h_mm: 30 },
    { label: fr.presetChildSpeech, text: fr.presetChildSpeechText, font: "Andika", size_pt: 14, align: "Left", italic: true, h_mm: 20 },
    // À repasser : grandes lettres cursives, gris clair ou creuses.
    { label: fr.presetTrace, text: fr.presetTraceText, font: "Playwrite FR Moderne", size_pt: 40, align: "Center", italic: false, h_mm: 36, color: "#c4c4c4" },
    { label: fr.presetHollow, text: fr.presetTraceText, font: "Playwrite FR Moderne", size_pt: 40, align: "Center", italic: false, h_mm: 36, outline: true },
  ];

  const run = (text: string, italic: boolean) => ({ text, bold: false, italic, underline: false, color: null, font: null, size_pt: null });

  /** Pose le texte ; dans un modèle, il est à remplir (son texte devient l'invite), ce qui se change dans sa barre. */
  function placeText(el: Element) {
    const placed = editor.templateMode ? withTemplateRole(el, "fill") : el;
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, placed] }));
  }

  /** Ajoute la zone au centre de la page, la sélectionne et ouvre l'écriture. */
  function add(preset?: Preset) {
    const el = textElement(centeredFrame(editor.page, preset ? 120 : 80, preset?.h_mm ?? 20));
    if (preset && el.kind.type === "Text") {
      const content: RichText = { paragraphs: [{ align: null, runs: [run(preset.text, preset.italic)] }] };
      el.kind = {
        ...el.kind,
        content,
        style: { ...el.kind.style, font: preset.font, size_pt: preset.size_pt, align: preset.align, color: preset.color ?? el.kind.style.color, outline: preset.outline ?? null },
      };
    }
    placeText(el);
    editor.selection = [el.id];
    editor.selectTextOnEdit = true;
    editor.editingText = el.id;
  }

  /** Un cadre de lignes d'écriture sur la largeur de la page, à écrire à la main ou à remplir. */
  function addLines(pattern: Pattern) {
    const el = textElement(centeredFrame(editor.page, editor.page.size.width_mm - 30, 48));
    if (el.kind.type === "Text") {
      el.kind = { ...el.kind, content: { paragraphs: [] }, style: { ...el.kind.style, font: "Playwrite FR Moderne", size_pt: 28 }, frame: { fill: null, stroke: null, corner_radius_mm: 0, image: null, pattern } };
    }
    placeText(el);
    editor.selection = [el.id];
  }

  /** Un champ : au curseur si l'on écrit, sinon un nouveau texte qui le contient, choisi. */
  function addField(key: string) {
    if (editor.editingText) {
      insertField(key);
      return;
    }
    const el = fieldText(editor.page, key);
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, el] }));
    editor.selection = [el.id];
  }

  let cols = $state(3);
  let rows = $state(3);

  /** Tableau de 150 × 90 mm au centre de la page, sélectionné en entier. */
  async function addTable() {
    const id = editor.page.id;
    try {
      const [page, table] = await tableAdd(editor.page, centeredFrame(editor.page, 150, 90), cols, rows);
      editor.updatePageById(id, () => page);
      const group = page.elements.find((e) => e.id === table)?.group;
      editor.selection = page.elements.filter((e) => e.group === group).map((e) => e.id);
    } catch (e) {
      editor.notice = String(e);
    }
  }
</script>

<Drawer title={fr.text}>
  <button class="primary" onclick={() => add()}>✏️ {fr.addText}</button>
  <!-- Sections repliées au départ : on voit d'un coup tout ce que le tiroir propose, et chacune reste comme on la
       laisse. -->
  <InspectorSection id="text-presets" title={fr.textPresets} open={false}>
    <div class="stack">
    {#each presets as p (p.label)}
      <button
        class="preset"
        class:hollow={p.outline}
        onclick={() => add(p)}
        style:font-family={`"${p.font}"`}
        style:font-size={`${Math.min(p.size_pt, 20)}px`}
        style:font-style={p.italic ? "italic" : "normal"}
        style:color={p.color}
      >
        {p.label}
      </button>
    {/each}
    </div>
  </InspectorSection>
  <InspectorSection id="text-fields" title={fr.fieldsTitle} open={false}>
    <p class="hint">{fr.fieldsHint}</p>
    <FieldMenu onpick={addField} />
  </InspectorSection>
  <InspectorSection id="text-lines" title={fr.writingLines} open={false}>
    <div class="lines">
      <button onclick={() => addLines("Seyes")}>{fr.patternSeyes}</button>
      <button onclick={() => addLines("Lines")}>{fr.patternLines}</button>
      <button onclick={() => addLines("SmallSquares")}>{fr.patternSquares}</button>
    </div>
  </InspectorSection>
  <InspectorSection id="text-table" title={fr.table} open={false}>
    <div class="table-size">
      <label>{fr.tableCols} <input type="number" min="1" max="8" bind:value={cols} /></label>
      <label>{fr.tableRows} <input type="number" min="1" max="8" bind:value={rows} /></label>
    </div>
    <button onclick={addTable}>▦ {fr.addTable}</button>
  </InspectorSection>
</Drawer>

<style>
  .stack {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .lines {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
    font-weight: 600;
    padding: 8px;
  }
  .table-size {
    display: flex;
    gap: 12px;
  }
  .table-size input {
    width: 3.5em;
  }
  .preset.hollow {
    -webkit-text-fill-color: transparent;
    -webkit-text-stroke: 0.6px currentColor;
  }
  .hint {
    margin: 0;
    color: var(--muted);
    font-size: 0.9em;
  }
  .preset {
    text-align: left;
    padding: 10px 12px;
  }
</style>
