<script lang="ts">
  // Poignées d'un groupe ou d'une sélection de plusieurs éléments : déplacer, agrandir (proportionnel,
  // depuis les coins ; un tableau s'étire librement, aussi par les bords) et tourner autour du centre. Pendant le geste seul le DOM bouge ;
  // à la fin, une seule modification du document (un pas d'annulation).
  import { inTableGroup, resizeTable, selectedTable } from "../lib/table";
  import Moveable from "moveable";
  import { onDestroy, tick } from "svelte";
  import { editor } from "../lib/docStore.svelte";
  import { applyGroupTransform } from "../lib/editActions";
  import { GUIDE_MARGIN_MM, SNAP_MM, settleMove, settleScale, snapLines } from "../lib/geometry";
  import { boundsOf, isLockedSelection } from "../lib/groups";
  import { PX_PER_MM, mmToPx } from "../lib/units";

  let { pageEl, zoom, onpick }: { pageEl: HTMLElement | undefined; zoom: number; onpick: (id: string | null, add: boolean) => void } = $props();

  let moveable: Moveable | null = null;
  let key = "";
  let gesture: { ids: string[]; box: { x_mm: number; y_mm: number; w_mm: number; h_mm: number }; styles: string[]; dx: number; dy: number; factor: number; w_px: number; h_px: number; dir: number[]; deg: number } | null = null;

  /** Plus petite case d'un tableau étiré (mm). */
  const MIN_CELL_MM = 5;

  const inGrid = (id: string) => !!editor.page.grid?.cells.some((c) => c.element_id === id);

  const ids = $derived.by(() => {
    const sel = editor.selection;
    if (sel.length < 2 || editor.cropping || editor.editingText) return null;
    if (sel.some(inGrid) || isLockedSelection(editor.page, sel)) return null;
    return [...sel];
  });

  // Tableau sélectionné en entier : il s'étire librement.
  const table = $derived(ids ? selectedTable(editor.page, ids) : undefined);

  function begin(targets: HTMLElement[]) {
    if (!ids) return;
    editor.gesture = true;
    const frames = editor.page.elements.filter((e) => ids.includes(e.id)).map((e) => e.frame);
    gesture = { ids, box: boundsOf(frames), styles: targets.map((t) => t.style.cssText), dx: 0, dy: 0, factor: 1, w_px: 0, h_px: 0, dir: [1, 1], deg: 0 };
  }

  function end(targets: HTMLElement[]) {
    editor.gesture = false;
    const g = gesture;
    gesture = null;
    if (!g) return;
    targets.forEach((t, i) => (t.style.cssText = g.styles[i]));
    const k = PX_PER_MM * zoom;
    const { box } = g;
    if (table?.kind.type === "Table" && g.w_px > 0) {
      stretchTable(g, table.id, table.kind.cols.length, table.kind.rows.length, k);
      return;
    }
    const changed = g.dx !== 0 || g.dy !== 0 || Math.abs(g.factor - 1) > 0.001 || Math.abs(g.deg) > 0.05;
    if (!changed) return;
    // Comme pour un bloc seul : poser exactement sur la ligne touchée à un pixel près.
    const lines = snapLines(
      editor.page.size,
      editor.page.elements.filter((e) => !g.ids.includes(e.id)).map((e) => boundsOf([e.frame])),
    );
    const tol = 1 / k + 0.05;
    let dx_mm = g.dx / k;
    let dy_mm = g.dy / k;
    let factor = g.factor;
    if (Math.abs(g.deg) <= 0.05 && Math.abs(g.factor - 1) <= 0.001) {
      const moved = { ...box, x_mm: box.x_mm + dx_mm, y_mm: box.y_mm + dy_mm, rotation_deg: 0 };
      const settled = settleMove(moved, lines, tol);
      dx_mm += settled.x_mm - moved.x_mm;
      dy_mm += settled.y_mm - moved.y_mm;
    } else if (Math.abs(g.deg) <= 0.05) factor = settleScale(box, factor, g.dir, lines, tol);
    applyGroupTransform(g.ids, {
      dx_mm,
      dy_mm,
      factor,
      // Agrandir depuis un coin : le coin opposé ne bouge pas.
      origin: { x_mm: g.dir[0] < 0 ? box.x_mm + box.w_mm : box.x_mm, y_mm: g.dir[1] < 0 ? box.y_mm + box.h_mm : box.y_mm },
      deg: Math.round(g.deg * 10) / 10,
      center: { x_mm: box.x_mm + box.w_mm / 2, y_mm: box.y_mm + box.h_mm / 2 },
    });
  }

  /** Nouveau cadre du tableau étiré : le bord ou le coin opposé à la poignée ne bouge pas. */
  function stretchTable(g: NonNullable<typeof gesture>, id: string, cols: number, rows: number, k: number) {
    const { box, dir } = g;
    const w = Math.max(cols * MIN_CELL_MM, Math.round((g.w_px / k) * 10) / 10);
    const h = Math.max(rows * MIN_CELL_MM, Math.round((g.h_px / k) * 10) / 10);
    if (Math.abs(w - box.w_mm) < 0.05 && Math.abs(h - box.h_mm) < 0.05) return;
    const frame = { x_mm: dir[0] < 0 ? box.x_mm + box.w_mm - w : box.x_mm, y_mm: dir[1] < 0 ? box.y_mm + box.h_mm - h : box.y_mm, w_mm: w, h_mm: h, rotation_deg: 0 };
    editor.updatePage((p) => resizeTable(p, id, frame));
  }

  function create(targets: HTMLElement[]): Moveable {
    const m = new Moveable(pageEl!, {
      target: targets,
      draggable: true,
      resizable: true,
      // Un tableau ne tourne pas : sa grille et son export restent droits.
      rotatable: !ids?.some((id) => inTableGroup(editor.page, id)),
      keepRatio: !table,
      renderDirections: table ? ["nw", "n", "ne", "w", "e", "sw", "s", "se"] : ["nw", "ne", "sw", "se"],
      snappable: true,
      isDisplaySnapDigit: false,
      snapThreshold: 6,
      snapDirections: { left: true, top: true, right: true, bottom: true, center: true, middle: true },
      elementSnapDirections: { left: true, top: true, right: true, bottom: true, center: true, middle: true },
      origin: false,
      // Le cadre du groupe reste droit : l'angle mesuré est celui du geste en cours.
      useDefaultGroupRotate: true,
      defaultGroupRotate: 0,
    });
    m.on("dragGroupStart", () => begin(targets))
      .on("dragGroup", ({ events }) => {
        events.forEach((ev) => (ev.target.style.transform = ev.transform));
        if (gesture) [gesture.dx, gesture.dy] = events[0].beforeTranslate;
      })
      .on("dragGroupEnd", () => end(targets))
      .on("resizeGroupStart", (ev) => {
        begin(targets);
        ev.events.forEach((e) => e.setOrigin(["%", "%"]));
      })
      .on("resizeGroup", (ev) => {
        ev.events.forEach((e) => {
          e.target.style.width = `${e.width}px`;
          e.target.style.height = `${e.height}px`;
          e.target.style.transform = e.drag.transform;
        });
        if (gesture) {
          gesture.factor = ev.width / (gesture.box.w_mm * PX_PER_MM * zoom);
          gesture.w_px = ev.width;
          gesture.h_px = ev.height;
          gesture.dir = ev.direction;
        }
      })
      .on("resizeGroupEnd", () => end(targets))
      .on("rotateGroupStart", () => begin(targets))
      .on("rotateGroup", (ev) => {
        ev.events.forEach((e) => (e.target.style.transform = e.drag.transform));
        if (gesture) gesture.deg = ev.beforeDist;
      })
      .on("rotateGroupEnd", () => end(targets))
      // La zone du groupe recouvre la boîte englobante : un clic sur un autre élément (ou sur la page)
      // dessous le sélectionne. Le double-clic est traité par la page.
      .on("clickGroup", (ev) => {
        if (ev.isDouble) return;
        const input = ev.inputEvent as MouseEvent;
        const id = document.elementsFromPoint(input.clientX, input.clientY).map((n) => n.closest<HTMLElement>(".element")).find((n) => n && pageEl?.contains(n))?.dataset.id ?? null;
        if (id && ids?.includes(id)) return;
        onpick(id, input.shiftKey);
      });
    return m;
  }

  // Après chaque changement du document, du zoom ou de la sélection : recréer les poignées.
  $effect(() => {
    void editor.doc;
    void zoom;
    const list = ids;
    const container = pageEl;
    tick().then(() => {
      if (!container || gesture) return;
      const targets = (list ?? []).map((id) => container.querySelector<HTMLElement>(`[data-id="${CSS.escape(id)}"]`)).filter((t): t is HTMLElement => !!t);
      const nextKey = targets.length > 1 ? list!.join(",") : "";
      if (!nextKey) {
        moveable?.destroy();
        moveable = null;
        key = "";
        return;
      }
      if (!moveable || key !== nextKey) {
        moveable?.destroy();
        moveable = create(targets);
        key = nextKey;
      }
      const w = mmToPx(editor.page.size.width_mm, zoom);
      const h = mmToPx(editor.page.size.height_mm, zoom);
      const m = mmToPx(GUIDE_MARGIN_MM, zoom);
      moveable.snapGridWidth = mmToPx(SNAP_MM, zoom);
      moveable.snapGridHeight = mmToPx(SNAP_MM, zoom);
      moveable.elementGuidelines = [...container.querySelectorAll<HTMLElement>(".element")].filter((n) => !targets.includes(n));
      moveable.verticalGuidelines = [0, m, w / 2, w - m, w];
      moveable.horizontalGuidelines = [0, m, h / 2, h - m, h];
      moveable.updateRect();
    });
  });

  onDestroy(() => moveable?.destroy());
</script>
