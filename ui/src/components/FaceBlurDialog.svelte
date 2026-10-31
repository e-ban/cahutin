<script lang="ts">
  // Flouter des visages : la photo entière en grand ; les visages repérés (si le repérage est actif) se
  // cochent d'un clic, d'autres zones se tracent à la main en glissant. « Flouter » remplace la photo par sa
  // version floutée partout dans le document, en un pas d'annulation. Cahutin ne sait jamais qui est qui.
  import { assetUrl, photoBlur, photoDetectFaces } from "../lib/api";
  import { editor } from "../lib/docStore.svelte";
  import { replaceAsset, replaceInLibrary, toggleZone, zoneFromDrag } from "../lib/faceBlur";
  import { fr } from "../lib/i18n/fr";
  import { readPref, writePref } from "../lib/prefs";
  import type { FaceBox } from "../lib/types";

  let { assetId, onclose }: { assetId: string; onclose: () => void } = $props();

  const meta = $derived(editor.assetMeta(assetId));
  let auto = $state(readPref("faceDetection", true));
  let detected = $state<FaceBox[]>([]);
  let detecting = $state(false);
  let chosen = $state(new Set<number>());
  let manual = $state<FaceBox[]>([]);
  let drag = $state<{ x: number; y: number; x1: number; y1: number } | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);
  let layerEl = $state<HTMLDivElement>();

  async function detect() {
    detecting = true;
    try {
      detected = await photoDetectFaces(assetId);
    } catch (e) {
      error = String(e);
    } finally {
      detecting = false;
    }
  }
  // Au premier affichage seulement : le réglage du poste dit s'il faut repérer.
  $effect(() => {
    if (readPref("faceDetection", true)) void detect();
  });

  function setAuto(on: boolean) {
    auto = on;
    writePref("faceDetection", on);
    if (on && !detected.length) void detect();
    if (!on) chosen = new Set();
  }

  /** Position du pointeur en fraction de l'image. */
  function at(ev: PointerEvent): { x: number; y: number } {
    const r = layerEl!.getBoundingClientRect();
    return { x: (ev.clientX - r.left) / r.width, y: (ev.clientY - r.top) / r.height };
  }

  /** Début d'une zone tracée à la main, seulement sur la photo : un appui sur un visage ou sur × ne doit pas
   *  capturer le pointeur, sinon leur clic n'arrive jamais. */
  function down(ev: PointerEvent) {
    if (ev.target !== layerEl) return;
    const p = at(ev);
    drag = { x: p.x, y: p.y, x1: p.x, y1: p.y };
    layerEl!.setPointerCapture(ev.pointerId);
  }
  function move(ev: PointerEvent) {
    if (!drag) return;
    const p = at(ev);
    drag = { ...drag, x1: p.x, y1: p.y };
  }
  function up() {
    if (!drag) return;
    const z = zoneFromDrag(drag.x, drag.y, drag.x1, drag.y1);
    if (z) manual = [...manual, z];
    drag = null;
  }

  // Écouteurs natifs sur le calque, plutôt que la délégation de Svelte.
  $effect(() => {
    const el = layerEl;
    if (!el) return;
    el.addEventListener("pointerdown", down);
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
    return () => {
      el.removeEventListener("pointerdown", down);
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
    };
  });

  const zones = $derived([...(auto ? detected.filter((_, i) => chosen.has(i)) : []), ...manual]);

  async function apply() {
    if (!zones.length || !meta) return;
    busy = true;
    error = null;
    try {
      const blurred = await photoBlur(assetId, zones);
      editor.registerAssets([blurred]);
      editor.commit(replaceAsset(editor.doc, assetId, blurred));
      editor.photos = replaceInLibrary(editor.photos, assetId, blurred);
      editor.notice = fr.blurDone(zones.length);
      onclose();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  const box = (z: FaceBox) => `left:${z.x * 100}%;top:${z.y * 100}%;width:${z.w * 100}%;height:${z.h * 100}%`;
  const preview = $derived(drag ? zoneFromDrag(drag.x, drag.y, drag.x1, drag.y1) : null);
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && !busy && onclose()} />

<div class="backdrop">
  <div class="box" role="dialog" aria-modal="true" aria-labelledby="blur-title">
    <h2 id="blur-title">{fr.blurTitle}</h2>
    <p class="hint">{auto ? fr.blurHintAuto : fr.blurHintManual}</p>
    {#if meta}
      <div class="stage" style:aspect-ratio="{meta.width_px} / {meta.height_px}" style:width="min(100%, calc(70vh * {meta.width_px / meta.height_px}))">
        <img src={assetUrl(assetId)} alt="" draggable="false" />
        <!-- Calque des zones : glisser sur la photo trace une zone ; chaque visage repéré est un bouton. -->
        <div class="layer" bind:this={layerEl} role="presentation">
          {#if auto}
            {#each detected as z, i (i)}
              <button
                class="face"
                class:on={chosen.has(i)}
                style={box(z)}
                aria-pressed={chosen.has(i)}
                aria-label={fr.blurFace(i + 1)}
                onclick={() => (chosen = toggleZone(chosen, i))}
              ></button>
            {/each}
          {/if}
          {#each manual as z, i (i)}
            <div class="face on manual" style={box(z)}></div>
            <button class="remove" style:left="{(z.x + z.w) * 100}%" style:top="{z.y * 100}%" title={fr.blurRemoveZone} aria-label={fr.blurRemoveZone} onclick={() => (manual = manual.filter((_, j) => j !== i))}>×</button>
          {/each}
          {#if preview}<div class="face drawing" style={box(preview)}></div>{/if}
        </div>
        {#if detecting}<div class="busy">{fr.blurDetecting}</div>{/if}
      </div>
    {/if}
    <label class="auto">
      <input type="checkbox" checked={auto} onchange={(e) => setAuto(e.currentTarget.checked)} />
      {fr.blurAuto}
    </label>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <div class="buttons">
      <button onclick={onclose} disabled={busy}>{fr.cancel}</button>
      <button class="primary" disabled={busy || zones.length === 0} onclick={apply}>{fr.blurApply(zones.length)}</button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(40, 30, 20, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 6000;
  }
  .box {
    background: var(--panel);
    border-radius: 12px;
    padding: 18px 22px;
    width: min(92vw, 1100px);
    max-height: 94vh;
    display: flex;
    flex-direction: column;
    gap: 10px;
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.25);
  }
  h2 {
    margin: 0;
    font-size: 18px;
  }
  p {
    margin: 0;
  }
  .hint {
    font-size: 13px;
    color: var(--muted);
  }
  .stage {
    position: relative;
    margin: 0 auto;
    user-select: none;
  }
  .stage img,
  .layer {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .layer {
    cursor: crosshair;
  }
  .face {
    position: absolute;
    box-sizing: border-box;
    padding: 0;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.05);
    border: 2px dashed white;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.5);
    cursor: pointer;
  }
  .face.on {
    background: rgba(46, 134, 222, 0.45);
    border: 2px solid #2e86de;
  }
  .face.drawing {
    background: rgba(46, 134, 222, 0.25);
    border-color: #2e86de;
    pointer-events: none;
  }
  .face.manual {
    cursor: default;
    pointer-events: none;
  }
  .remove {
    position: absolute;
    transform: translate(-50%, -50%);
    width: 22px;
    height: 22px;
    padding: 0;
    border-radius: 50%;
    background: white;
    line-height: 1;
  }
  .busy {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    background: rgba(255, 255, 255, 0.9);
    padding: 6px 12px;
    border-radius: 6px;
  }
  .auto {
    display: flex;
    gap: 6px;
    align-items: center;
    font-size: 13px;
  }
  .error {
    color: #c0392b;
  }
  .buttons {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
  .primary {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
</style>
