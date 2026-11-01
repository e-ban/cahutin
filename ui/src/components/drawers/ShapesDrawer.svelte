<script lang="ts">
  import Drawer from "../layout/Drawer.svelte";
  import ShapeEl from "../elements/ShapeEl.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { centeredFrame, shapeElement } from "../../lib/factory";
  import { MOTIFS, PATHS, dottedPath, graphismeRows, motifPreview, type Level, type Motif } from "../../lib/graphisme";
  import { readPref, writePref } from "../../lib/prefs";
  import { SYMMETRY_FIGURES, maze, symmetry } from "../../lib/fiches";
  import { fr } from "../../lib/i18n/fr";
  import type { Element, ShapeKind } from "../../lib/types";

  const shapes: { shape: ShapeKind; radius: number; label: string }[] = [
    { shape: "Rect", radius: 0, label: fr.rect },
    { shape: "Rect", radius: 6, label: fr.roundedRect },
    { shape: "Ellipse", radius: 0, label: fr.ellipse },
    { shape: "Line", radius: 0, label: fr.line },
    { shape: "Arrow", radius: 0, label: fr.arrow },
    { shape: "Star", radius: 0, label: fr.star },
    { shape: "Heart", radius: 0, label: fr.heart },
    { shape: "Bubble", radius: 0, label: fr.bubble },
    { shape: "Cloud", radius: 0, label: fr.cloud },
  ];

  // Aperçu dessiné par le même composant que la page.
  const previews: Element[] = shapes.map((s) => shapeElement(editor.page, s.shape, s.radius));

  // Lignes de graphisme : la section (taille des motifs) et le nombre de lignes, gardés d'une fois sur l'autre.
  let level = $state<Level>(readPref("graphismeLevel", "MS"));
  let rows = $state<number>(readPref("graphismeRows", 4));
  $effect(() => writePref("graphismeLevel", level));
  $effect(() => writePref("graphismeRows", rows));

  const pathProps = (path: string) => ({ shape: "Path" as const, path, fill: null, stroke: { width_mm: 0.6, color: "#333333" }, corner_radius_mm: 0 });

  function addRows(m: Motif) {
    const els = graphismeRows(editor.page, m, level, rows);
    if (!els.length) {
      editor.notice = fr.graphismeNoRoom;
      return;
    }
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, ...els] }));
    editor.selection = els.map((e) => e.id);
  }

  /** Une fiche générée (labyrinthe, symétrie) sous le contenu de la page, ou un message s'il n'y a plus la place. */
  function addSheet(els: Element[]) {
    if (!els.length) {
      editor.notice = fr.graphismeNoRoom;
      return;
    }
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, ...els] }));
    editor.selection = els.map((e) => e.id);
  }

  /** Repères d'écriture : un point vert de départ ou une petite flèche verte de sens, à poser sur une lettre. */
  function addMarker(kind: "point" | "fleche") {
    const green = "#2e9d4a";
    const base = shapeElement(editor.page, kind === "point" ? "Ellipse" : "Arrow");
    const el: Element =
      kind === "point"
        ? { ...base, frame: centeredFrame(editor.page, 5, 5), kind: { type: "Shape", shape: "Ellipse", fill: green, stroke: null, corner_radius_mm: 0 } }
        : { ...base, frame: centeredFrame(editor.page, 14, 6), kind: { type: "Shape", shape: "Arrow", fill: null, stroke: { width_mm: 0.8, color: green, style: "Solid" }, corner_radius_mm: 0 } };
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, el] }));
    editor.selection = [el.id];
  }

  function addPath(path: string, w: number, h: number) {
    const el = dottedPath(path, centeredFrame(editor.page, w, h));
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, el] }));
    editor.selection = [el.id];
  }

  function add(shape: ShapeKind, radius: number) {
    const el = shapeElement(editor.page, shape, radius);
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, el] }));
    editor.selection = [el.id];
  }
</script>

<Drawer title={fr.shapes}>
  <div class="grid">
    {#each shapes as s, i (s.label)}
      {@const el = previews[i]}
      <button onclick={() => add(s.shape, s.radius)}>
        <span class="preview" style:width="{el.frame.w_mm * 0.75}px" style:height="{el.frame.h_mm * 0.75}px">
          {#if el.kind.type === "Shape"}<ShapeEl props={el.kind} frame={el.frame} zoom={0.75 / 3.7795} />{/if}
        </span>
        {s.label}
      </button>
    {/each}
  </div>

  <h3>{fr.graphisme}</h3>
  <div class="options">
    <label>{fr.graphismeLevel}
      <select bind:value={level}>
        <option value="PS">PS</option>
        <option value="MS">MS</option>
        <option value="GS">GS</option>
      </select>
    </label>
    <label>{fr.graphismeRows} <input type="number" min="1" max="12" bind:value={rows} /></label>
  </div>
  <div class="grid motifs">
    {#each MOTIFS as m (m)}
      {@const pv = motifPreview(m)}
      {@const px = Math.min(2, 56 / pv.frame.w_mm)}
      <button title={fr.graphismeAdd(fr.motifs[m])} onclick={() => addRows(m)}>
        <span class="preview" style:width="{pv.frame.w_mm * px}px" style:height="{pv.frame.h_mm * px}px">
          <ShapeEl props={pathProps(pv.path)} frame={pv.frame} zoom={px / 3.7795} />
        </span>
        {fr.motifs[m]}
      </button>
    {/each}
  </div>
  <p class="hint">{fr.graphismeHint}</p>

  <h3>{fr.mazeTitle}</h3>
  <button class="wide" onclick={() => addSheet(maze(editor.page, level))}>🐤 {fr.mazeAdd(level)}</button>
  <p class="hint">{fr.mazeHint}</p>

  <h3>{fr.symmetryTitle}</h3>
  <div class="grid">
    {#each Object.keys(SYMMETRY_FIGURES) as f (f)}
      <button onclick={() => addSheet(symmetry(editor.page, f, level))}>{fr.symmetryFigures[f]}</button>
    {/each}
  </div>
  <p class="hint">{fr.symmetryHint}</p>

  <h3>{fr.markersTitle}</h3>
  <div class="grid paths">
    <button onclick={() => addMarker("point")}><span class="dot"></span>{fr.markerStart}</button>
    <button onclick={() => addMarker("fleche")}><span class="arrow">➜</span>{fr.markerArrow}</button>
  </div>
  <p class="hint">{fr.markersHint}</p>

  <h3>{fr.paths}</h3>
  <div class="grid paths">
    {#each PATHS as p (p.id)}
      <button onclick={() => addPath(p.path, p.w_mm, p.h_mm)}>
        <span class="preview" style:width="{p.w_mm * 0.5}px" style:height="{p.h_mm * 0.5}px">
          <ShapeEl props={{ ...pathProps(p.path), stroke: { width_mm: 1.2, color: "#555555", style: "Dotted" } }} frame={{ x_mm: 0, y_mm: 0, w_mm: p.w_mm, h_mm: p.h_mm, rotation_deg: 0 }} zoom={0.5 / 3.7795} />
        </span>
        {fr.pathNames[p.id]}
      </button>
    {/each}
  </div>
  <p class="hint">{fr.pathsHint}</p>
</Drawer>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }
  .grid button {
    padding: 10px 4px;
    font-size: 12px;
    min-width: 0;
  }
  .grid button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }
  h3 {
    margin: 14px 0 6px;
  }
  .options {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 12px;
    margin-bottom: 8px;
    font-size: 13px;
  }
  .options label {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .options input {
    width: 4em;
    box-sizing: border-box;
  }
  .hint {
    color: var(--muted);
    font-size: 12px;
    margin: 6px 0 0;
  }
  .wide {
    width: 100%;
    padding: 8px;
  }
  .dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #2e9d4a;
    margin-top: 4px;
  }
  .arrow {
    color: #2e9d4a;
    font-size: 18px;
  }
  .paths {
    grid-template-columns: repeat(2, 1fr);
  }
  .preview {
    position: relative;
    display: block;
    margin: 4px auto 0;
  }
</style>
