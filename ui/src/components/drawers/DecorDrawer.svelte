<script lang="ts">
  import Icon from "../Icon.svelte";
  import GuideLink from "../GuideLink.svelte";
  // Tiroir Décors : cadres de page et frises. Les motifs d'une frise se choisissent dans une fenêtre ; les
  // images à poser sont dans le tiroir Images.
  import Drawer from "../layout/Drawer.svelte";
  import FriezePicker from "../FriezePicker.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import InspectorSection from "../layout/InspectorSection.svelte";
  import { frieze, pageFrame, type FrameStyle, type FriezeSide } from "../../lib/decor";
  import { friezeMotifs, motifUrl, resolveMotifs } from "../../lib/friezeMotifs.svelte";
  import { readPref, writePref } from "../../lib/prefs";
  import type { AssetMeta, Element } from "../../lib/types";

  const frames: { style: FrameStyle; label: string }[] = [
    { style: "thin", label: fr.frameThin },
    { style: "thick", label: fr.frameThick },
    { style: "rounded", label: fr.frameRounded },
    { style: "double", label: fr.frameDouble },
  ];
  const sides: { side: FriezeSide; label: string }[] = [
    { side: "top", label: fr.friezeTop },
    { side: "bottom", label: fr.friezeBottom },
    { side: "around", label: fr.friezeAround },
  ];

  /** Cadre : derrière tout le reste ; frise : devant. Les nouveaux éléments sont sélectionnés. */
  function addDecor(els: Element[], behind: boolean, assets: AssetMeta[] = []) {
    editor.updatePage((p) => ({ ...p, elements: behind ? [...els, ...p.elements] : [...p.elements, ...els] }), assets);
    editor.selection = els.map((e) => e.id);
  }

  async function addFrieze(side: FriezeSide) {
    const { motifs, assets } = await resolveMotifs(friezeMotifs.codes);
    if (motifs.length) addDecor(frieze(editor.page, motifs, side, friezeMotifs.size, friezeMotifs.gap), false, assets);
  }

  let picking = $state(false);
  // Les pictogrammes étaient ici jusqu'à la 0.9 : on le dit, jusqu'à ce que le message soit fermé.
  let movedSeen = $state(readPref("picturesMovedSeen", false));
  function closeMoved() {
    movedSeen = true;
    writePref("picturesMovedSeen", true);
  }
</script>

<Drawer title={fr.decor}>
  {#if !movedSeen}
    <div class="moved" role="note">
      <span>{fr.picturesMoved}</span>
      <button class="link" onclick={() => (closeMoved(), editor.openImages("pictos"))}>{fr.picturesMovedOpen}</button>
      <button class="close" title={fr.close} aria-label={fr.close} onclick={closeMoved}><Icon name="close" size={14} /></button>
    </div>
  {/if}
  <InspectorSection id="decor-frames" title={fr.frames} hint={fr.sectionHints.frames} open={false}>
    <div class="frames">
      {#each frames as f (f.style)}
        <button onclick={() => addDecor(pageFrame(editor.page, f.style), true)}><i class="mini {f.style}"></i>{f.label}</button>
      {/each}
    </div>
  </InspectorSection>
  <InspectorSection id="decor-friezes" title={fr.friezes} hint={fr.sectionHints.friezes} open={false}>
    <div><GuideLink id="frise" /></div>
    <span class="label">{fr.friezeMotifs}</span>
    <div class="motifs">
      {#each friezeMotifs.codes as code, i (code)}
        <span class="motif">
          <img src={motifUrl(code)} alt="" />
          <button class="remove" title={fr.removeMotif} aria-label={fr.removeMotif} onclick={() => (friezeMotifs.codes = friezeMotifs.codes.filter((_, j) => j !== i))}><Icon name="close" size={12} /></button>
        </span>
      {:else}
        <span class="hint">{fr.friezeNoMotif}</span>
      {/each}
    </div>
    <button data-guide="frieze-pick" onclick={() => (picking = true)}><Icon name="add" /> {fr.friezePick}</button>
    <label class="slider">
      <span>{fr.friezeSize}</span>
      <input type="range" min="6" max="40" step="1" bind:value={friezeMotifs.size} />
      <span class="value">{friezeMotifs.size} mm</span>
    </label>
    <label class="slider">
      <span>{fr.friezeGap}</span>
      <input type="range" min="0" max="30" step="1" bind:value={friezeMotifs.gap} />
      <span class="value">{friezeMotifs.gap} mm</span>
    </label>
    <div class="sides">
      {#each sides as s (s.side)}
        <button disabled={friezeMotifs.codes.length === 0} onclick={() => addFrieze(s.side)}>{s.label}</button>
      {/each}
    </div>
    <p class="hint">{fr.friezeEditHint}</p>
  </InspectorSection>
</Drawer>
{#if picking}<FriezePicker bind:codes={friezeMotifs.codes} onclose={() => (picking = false)} />{/if}

<style>
  .moved {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 8px;
    padding: 8px 28px 8px 10px;
    position: relative;
    border-radius: 8px;
    background: var(--warn-bg);
    border: 1px solid var(--warn-line);
    font-size: 13px;
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    text-decoration: underline;
    font-size: 13px;
  }
  .close {
    position: absolute;
    top: 4px;
    right: 4px;
    width: 20px;
    height: 20px;
    padding: 0;
    border: 0;
    background: none;
    font-size: 16px;
    line-height: 18px;
  }
  .frames {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 4px;
  }
  .frames button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 6px 2px;
    font-size: 11px;
  }
  .mini {
    display: block;
    width: 26px;
    height: 36px;
    border: 1px solid #555;
    box-sizing: border-box;
  }
  .mini.thick {
    border-width: 3px;
  }
  .mini.rounded {
    border-radius: 5px;
    border-width: 2px;
  }
  .mini.double {
    border-style: double;
    border-width: 4px;
  }
  .label {
    font-size: 12px;
    color: var(--muted);
  }
  .motifs {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    min-height: 34px;
  }
  .motif {
    position: relative;
  }
  .motif img {
    width: 32px;
    height: 32px;
    display: block;
  }
  .remove {
    position: absolute;
    top: -6px;
    right: -6px;
    width: 16px;
    height: 16px;
    padding: 0;
    border-radius: 50%;
    font-size: 11px;
    line-height: 14px;
  }
  .slider {
    display: grid;
    grid-template-columns: 6.5em 1fr 3.5em;
    align-items: center;
    gap: 6px;
    font-size: 13px;
  }
  .slider input {
    min-width: 0;
  }
  .value {
    text-align: right;
    color: var(--muted);
  }
  .sides {
    display: flex;
    gap: 4px;
  }
  .sides button {
    flex: 1;
    padding: 6px 2px;
    font-size: 12px;
  }
  .hint {
    color: var(--muted);
    font-size: 12px;
    margin: 0;
  }
</style>
