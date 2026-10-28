<script lang="ts">
  import Drawer from "../layout/Drawer.svelte";
  import { editor } from "../../lib/docStore.svelte";
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
  }

  const presets: Preset[] = [
    { label: fr.presetTitle, text: fr.presetTitleText, font: "Playwrite FR Moderne", size_pt: 24, align: "Center", italic: false, h_mm: 22 },
    { label: fr.presetParagraph, text: fr.presetParagraphText, font: "Andika", size_pt: 14, align: "Left", italic: false, h_mm: 30 },
    { label: fr.presetChildSpeech, text: fr.presetChildSpeechText, font: "Andika", size_pt: 14, align: "Left", italic: true, h_mm: 20 },
  ];

  const run = (text: string, italic: boolean) => ({ text, bold: false, italic, underline: false, color: null, font: null, size_pt: null });

  /** Ajoute la zone au centre de la page, la sélectionne et ouvre l'écriture. */
  function add(preset?: Preset) {
    const el = textElement(centeredFrame(editor.page, preset ? 120 : 80, preset?.h_mm ?? 20));
    if (preset && el.kind.type === "Text") {
      const content: RichText = { paragraphs: [{ align: null, runs: [run(preset.text, preset.italic)] }] };
      el.kind = { ...el.kind, content, style: { ...el.kind.style, font: preset.font, size_pt: preset.size_pt, align: preset.align } };
    }
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, el] }));
    editor.selection = [el.id];
    editor.editingText = el.id;
  }
</script>

<Drawer title={fr.text}>
  <button class="primary" onclick={() => add()}>✏️ {fr.addText}</button>
  <h3>{fr.textPresets}</h3>
  {#each presets as p (p.label)}
    <button class="preset" onclick={() => add(p)} style:font-family={`"${p.font}"`} style:font-size={`${Math.min(p.size_pt, 20)}px`} style:font-style={p.italic ? "italic" : "normal"}>
      {p.label}
    </button>
  {/each}
</Drawer>

<style>
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
    font-weight: 600;
    padding: 8px;
  }
  .preset {
    text-align: left;
    padding: 10px 12px;
  }
</style>
