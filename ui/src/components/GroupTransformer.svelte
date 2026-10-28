<script lang="ts">
  // Poignées d'un groupe ou d'une sélection de plusieurs éléments : déplacer, agrandir (toujours
  // proportionnel, depuis les coins) et tourner autour du centre. Pendant le geste seul le DOM bouge ;
  // à la fin, une seule modification du document (un pas d'annulation).
  import Moveable from "moveable";
  import { onDestroy, tick } from "svelte";
  import { editor } from "../lib/docStore.svelte";
  import { applyGroupTransform } from "../lib/editActions";
  import { boundsOf, isLockedSelection } from "../lib/groups";
  import { PX_PER_MM } from "../lib/units";

  let { pageEl, zoom }: { pageEl: HTMLElement | undefined; zoom: number } = $props();

  let moveable: Moveable | null = null;
  let key = "";
  let gesture: { ids: string[]; box: { x_mm: number; y_mm: number; w_mm: number; h_mm: number }; styles: string[]; dx: number; dy: number; factor: number; dir: number[]; deg: number } | null = null;

  const inGrid = (id: string) => !!editor.page.grid?.cells.some((c) => c.element_id === id);

  const ids = $derived.by(() => {
    const sel = editor.selection;
    if (sel.length < 2 || editor.cropping || editor.editingText) return null;
    if (sel.some(inGrid) || isLockedSelection(editor.page, sel)) return null;
    return [...sel];
  });

  function begin(targets: HTMLElement[]) {
    if (!ids) return;
    editor.gesture = true;
    const frames = editor.page.elements.filter((e) => ids.includes(e.id)).map((e) => e.frame);
    gesture = { ids, box: boundsOf(frames), styles: targets.map((t) => t.style.cssText), dx: 0, dy: 0, factor: 1, dir: [1, 1], deg: 0 };
  }

  function end(targets: HTMLElement[]) {
    editor.gesture = false;
    const g = gesture;
    gesture = null;
    if (!g) return;
    targets.forEach((t, i) => (t.style.cssText = g.styles[i]));
    const k = PX_PER_MM * zoom;
    const { box } = g;
    const changed = g.dx !== 0 || g.dy !== 0 || Math.abs(g.factor - 1) > 0.001 || Math.abs(g.deg) > 0.05;
    if (!changed) return;
    applyGroupTransform(g.ids, {
      dx_mm: g.dx / k,
      dy_mm: g.dy / k,
      factor: g.factor,
      // Agrandir depuis un coin : le coin opposé ne bouge pas.
      origin: { x_mm: g.dir[0] < 0 ? box.x_mm + box.w_mm : box.x_mm, y_mm: g.dir[1] < 0 ? box.y_mm + box.h_mm : box.y_mm },
      deg: Math.round(g.deg * 10) / 10,
      center: { x_mm: box.x_mm + box.w_mm / 2, y_mm: box.y_mm + box.h_mm / 2 },
    });
  }

  function create(targets: HTMLElement[]): Moveable {
    const m = new Moveable(pageEl!, {
      target: targets,
      draggable: true,
      resizable: true,
      rotatable: true,
      keepRatio: true,
      renderDirections: ["nw", "ne", "sw", "se"],
      origin: false,
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
          gesture.dir = ev.direction;
        }
      })
      .on("resizeGroupEnd", () => end(targets))
      .on("rotateGroupStart", () => begin(targets))
      .on("rotateGroup", (ev) => {
        ev.events.forEach((e) => (e.target.style.transform = e.drag.transform));
        if (gesture) gesture.deg = ev.beforeRotate;
      })
      .on("rotateGroupEnd", () => end(targets))
      // La zone du groupe recouvre ses éléments : un double-clic est transmis à l'élément visé.
      .on("clickGroup", (ev) => {
        if (ev.isDouble && ev.targetIndex >= 0) targets[ev.targetIndex].dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
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
      moveable.updateRect();
    });
  });

  onDestroy(() => moveable?.destroy());
</script>
