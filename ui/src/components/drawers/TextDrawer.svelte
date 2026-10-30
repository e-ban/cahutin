<script lang="ts">
  import Drawer from "../layout/Drawer.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { tableAdd } from "../../lib/api";
  import { centeredFrame, textElement } from "../../lib/factory";
  import { fr } from "../../lib/i18n/fr";
  import type { Align, RichText } from "../../lib/types";

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
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, el] }));
    editor.selection = [el.id];
    editor.editingText = el.id;
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
  <h3>{fr.textPresets}</h3>
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
  <h3>{fr.table}</h3>
  <div class="table-size">
    <label>{fr.tableCols} <input type="number" min="1" max="8" bind:value={cols} /></label>
    <label>{fr.tableRows} <input type="number" min="1" max="8" bind:value={rows} /></label>
  </div>
  <button onclick={addTable}>▦ {fr.addTable}</button>
</Drawer>

<style>
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
  .preset {
    text-align: left;
    padding: 10px 12px;
  }
</style>
