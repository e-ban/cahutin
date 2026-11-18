<script lang="ts">
  // Dessin à main levée : pendant que le crayon est pris (lib/pen.svelte.ts), appuyer sur la page et glisser trace
  // un chemin (lib/freehand.ts), un pas d'annulation par tracé. Un bouton, un menu ou Échap rangent le crayon ; la
  // marge grise ne fait rien.
  import { onDestroy } from "svelte";
  import { editor } from "../lib/docStore.svelte";
  import { strokeElement } from "../lib/freehand";
  import { fr } from "../lib/i18n/fr";
  import type { Pt } from "../lib/linePath";
  import { pen } from "../lib/pen.svelte";
  import { PX_PER_MM, mmToPx } from "../lib/units";

  let { pageEl, stageEl, zoom }: { pageEl: HTMLElement | undefined; stageEl: HTMLElement | undefined; zoom: number } = $props();

  /** Un point n'est gardé qu'à cette distance du précédent (mm). */
  const STEP_MM = 0.3;

  // Le tracé en cours : un tableau simple (pas de proxy) et un compteur qui redessine l'aperçu.
  let stroke: Pt[] | null = null;
  let drawn = $state(0);
  let pointer = -1;
  /** Bulle d'aide au premier usage du crayon dans la séance. */
  let hintShown = false;
  let hint = $state<{ x: number; y: number } | null>(null);
  let swallowUntil = 0;

  const toPage = (ev: { clientX: number; clientY: number }): Pt | null => {
    if (!pageEl) return null;
    const r = pageEl.getBoundingClientRect();
    return { x: (ev.clientX - r.left) / (PX_PER_MM * zoom), y: (ev.clientY - r.top) / (PX_PER_MM * zoom) };
  };
  const onPage = (t: EventTarget | null) => !!pageEl && t instanceof Node && pageEl.contains(t);
  const inStage = (t: EventTarget | null) => !!stageEl && t instanceof Node && stageEl.contains(t);
  const control = (t: EventTarget | null) => t instanceof Element && !!t.closest("button, input, select, textarea, a, [role='menu'], [role='dialog'], .pop");

  function stop() {
    pen.active = false;
    stroke = null;
    drawn++;
    hint = null;
  }

  /** Fin du tracé : il devient un chemin (un pas d'annulation), sauf s'il est abandonné. */
  function end(keep: boolean) {
    const done = stroke;
    stroke = null;
    drawn++;
    // Les clics qui suivent le relâcher (même longtemps après l'appui) ne vont pas à la page.
    swallowUntil = performance.now() + 400;
    if (!keep || !done) return;
    const el = strokeElement(editor.page, done);
    if (el) editor.updatePage((page) => ({ ...page, elements: [...page.elements, { ...el, locked: editor.templateMode }] }));
  }

  $effect(() => {
    if (!pen.active) return;
    if (!hintShown) {
      hintShown = true;
      hint = { x: 0, y: 0 };
    }
    const down = (ev: PointerEvent) => {
      if (ev.target instanceof Element && ev.target.closest("[data-pen-toggle]")) return;
      if (control(ev.target) || !inStage(ev.target)) return stop();
      ev.preventDefault();
      ev.stopPropagation();
      swallowUntil = performance.now() + 400;
      // Clic droit : le crayon se range (pas de menu sur l'élément dessous).
      if (ev.button === 2) return stop();
      // Un second doigt pendant un tracé ne le remplace pas.
      if (ev.button !== 0 || stroke) return;
      if (!onPage(ev.target)) return;
      const p = toPage(ev);
      if (!p) return;
      pointer = ev.pointerId;
      stroke = [p];
      drawn++;
      hint = null;
    };
    const moveAt = (ev: PointerEvent) => {
      if (hint) hint = { x: ev.clientX, y: ev.clientY };
      if (!stroke || ev.pointerId !== pointer) return;
      // Le relâcher s'est perdu (autre fenêtre) : le tracé finit là, il ne suit pas la souris sans bouton.
      if (ev.pointerType === "mouse" && ev.buttons === 0) return end(true);
      // Tous les points d'un geste rapide (sinon le tracé devient anguleux).
      for (const e of ev.getCoalescedEvents?.() ?? [ev]) {
        const p = toPage(e);
        const last = stroke[stroke.length - 1];
        if (p && Math.hypot(p.x - last.x, p.y - last.y) >= STEP_MM) stroke.push(p);
      }
      drawn++;
    };
    const up = (ev: PointerEvent) => {
      if (!stroke || ev.pointerId !== pointer) return;
      const p = toPage(ev);
      if (p) stroke.push(p);
      end(true);
    };
    // Geste interrompu (le navigateur reprend la main, la fenêtre perd le focus) : le tracé est abandonné.
    const cancel = (ev: PointerEvent) => ev.pointerId === pointer && end(false);
    const blur = () => end(false);
    const key = (ev: KeyboardEvent) => {
      // Une fenêtre de dialogue ouverte : son Échap d'abord.
      if (ev.key === "Escape" && !document.querySelector("[role='dialog']")) {
        ev.preventDefault();
        ev.stopPropagation();
        stop();
      }
    };
    window.addEventListener("pointerdown", down, true);
    window.addEventListener("pointermove", moveAt, true);
    window.addEventListener("pointerup", up, true);
    window.addEventListener("pointercancel", cancel, true);
    window.addEventListener("blur", blur);
    window.addEventListener("keydown", key, true);
    return () => {
      window.removeEventListener("pointerdown", down, true);
      window.removeEventListener("pointermove", moveAt, true);
      window.removeEventListener("pointerup", up, true);
      window.removeEventListener("pointercancel", cancel, true);
      window.removeEventListener("blur", blur);
      window.removeEventListener("keydown", key, true);
    };
  });

  // Les clics qui suivent un appui pris par le crayon (clic, double-clic, menu du clic droit) sont avalés, même
  // après que le crayon s'est rangé : ils arrivent après, et ouvriraient un menu ou sélectionneraient l'élément dessous.
  $effect(() => {
    const swallow = (ev: Event) => {
      if (performance.now() < swallowUntil) {
        ev.preventDefault();
        ev.stopPropagation();
      }
    };
    for (const name of ["click", "dblclick", "contextmenu"]) window.addEventListener(name, swallow, true);
    return () => {
      for (const name of ["click", "dblclick", "contextmenu"]) window.removeEventListener(name, swallow, true);
    };
  });

  // Le crayon pris, rien ne reste sélectionné (Ctrl+A, coller, annuler...) : des poignées s'afficheraient sans
  // pouvoir servir.
  $effect(() => {
    if (pen.active && editor.selection.length) editor.selection = [];
  });

  // Un autre document range le crayon.
  let lastGeneration = editor.generation;
  $effect(() => {
    if (editor.generation !== lastGeneration) {
      lastGeneration = editor.generation;
      if (pen.active) stop();
    }
  });

  // La couche disparaît (proposition de l'assistant...) : le crayon se range, la page retrouve son curseur.
  onDestroy(() => {
    if (pen.active) pen.active = false;
    stageEl?.classList.remove("pen");
  });

  // Une autre page range le crayon.
  let lastPage = editor.pageIndex;
  $effect(() => {
    if (editor.pageIndex !== lastPage) {
      lastPage = editor.pageIndex;
      if (pen.active) stop();
    }
  });

  $effect(() => {
    if (!stageEl) return;
    stageEl.classList.toggle("pen", pen.active);
  });

  // L'aperçu se redessine à chaque `drawn++` : ces deux fonctions le lisent pour que la page en dépende.
  const inking = (_version: number) => stroke !== null;
  const inkPoints = (_version: number) => (stroke ?? []).map((p) => `${mmToPx(p.x, zoom)},${mmToPx(p.y, zoom)}`).join(" ");

  /** Le tracé en cours vit dans la page : il suit son défilement et son zoom. */
  function inPage(node: Element) {
    pageEl?.appendChild(node);
    return { destroy: () => node.remove() };
  }
</script>

{#if pen.active && inking(drawn) && pageEl}
  <!-- Aperçu à l'épaisseur du trait posé (1 mm) : il ne change pas d'allure au relâcher. -->
  <svg class="ink" use:inPage aria-hidden="true">
    <polyline points={inkPoints(drawn)} stroke-width={mmToPx(1, zoom)} />
  </svg>
{/if}
{#if pen.active && hint && hint.x}
  <div class="hint" style:left="{hint.x + 16}px" style:top="{hint.y + 16}px">{fr.penHint}</div>
{/if}

<style>
  .ink {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: visible;
    pointer-events: none;
    z-index: 3050;
  }
  .ink polyline {
    fill: none;
    stroke: #333;
    stroke-linejoin: round;
    stroke-linecap: round;
  }
  .hint {
    position: fixed;
    z-index: 5000;
    pointer-events: none;
    background: var(--panel, #fff);
    color: var(--text, #222);
    border: 1px solid var(--border, #ccc);
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 13px;
    box-shadow: 0 2px 8px rgb(0 0 0 / 15%);
  }
  :global(.pen .page),
  :global(.pen .page *) {
    cursor: crosshair !important;
  }
  /* Au doigt ou au stylet, glisser sur la page dessine au lieu de la faire défiler. */
  :global(.pen .page) {
    touch-action: none;
  }
</style>
