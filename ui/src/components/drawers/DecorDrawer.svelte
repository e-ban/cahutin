<script lang="ts">
  // Tiroir Décors : cadres de page et frises. Les motifs d'une frise se choisissent dans une fenêtre ; les
  // images à poser sont dans le tiroir Images.
  import Drawer from "../layout/Drawer.svelte";
  import FriezePicker from "../FriezePicker.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { pictoUrl } from "../../lib/pictos.svelte";
  import { frieze, pageFrame, type FrameStyle, type FriezeSide } from "../../lib/decor";
  import { friezeMotifs } from "../../lib/friezeMotifs.svelte";
  import { readPref, writePref } from "../../lib/prefs";
  import type { Element } from "../../lib/types";

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
  function addDecor(els: Element[], behind: boolean) {
    editor.updatePage((p) => ({ ...p, elements: behind ? [...els, ...p.elements] : [...p.elements, ...els] }));
    editor.selection = els.map((e) => e.id);
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
      <button class="close" title={fr.close} aria-label={fr.close} onclick={closeMoved}>×</button>
    </div>
  {/if}
  <h3>{fr.frames}</h3>
  <div class="frames">
    {#each frames as f (f.style)}
      <button onclick={() => addDecor(pageFrame(editor.page, f.style), true)}><i class="mini {f.style}"></i>{f.label}</button>
    {/each}
  </div>

  <h3>{fr.friezes}</h3>
  <span class="label">{fr.friezeMotifs}</span>
  <div class="motifs">
    {#each friezeMotifs.codes as code, i (code)}
      <span class="motif">
        <img src={pictoUrl(code)} alt="" />
        <button class="remove" title={fr.removeMotif} aria-label={fr.removeMotif} onclick={() => (friezeMotifs.codes = friezeMotifs.codes.filter((_, j) => j !== i))}>×</button>
      </span>
    {:else}
      <span class="hint">{fr.friezeNoMotif}</span>
    {/each}
  </div>
  <button onclick={() => (picking = true)}>＋ {fr.friezePick}</button>
  <div class="sides">
    {#each sides as s (s.side)}
      <button disabled={friezeMotifs.codes.length === 0} onclick={() => addDecor(frieze(editor.page, friezeMotifs.codes, s.side), false)}>{s.label}</button>
    {/each}
  </div>
</Drawer>
{#if picking}<FriezePicker onclose={() => (picking = false)} />{/if}

<style>
  h3 {
    margin: 2px 0 0;
  }
  .moved {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 8px;
    padding: 8px 28px 8px 10px;
    position: relative;
    border-radius: 8px;
    background: #fff6e6;
    border: 1px solid #f0c98a;
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
