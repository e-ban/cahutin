<script lang="ts">
  // Poignées de déplacement, redimensionnement et rotation (Moveable) sur l'élément sélectionné.
  // Pendant le geste, seul le DOM bouge ; le document n'est modifié qu'à la fin du geste.
  // Une instance Moveable par cible : changer `target` d'une instance existante lui fait perdre
  // le geste en cours.
  import Moveable from "moveable";
  import { onDestroy, tick } from "svelte";
  import { editor } from "../lib/docStore.svelte";
  import { GUIDE_MARGIN_MM, SNAP_MM, rectPxToFrame, settleMove, settleResize, snapLines, type RectPx } from "../lib/geometry";
  import { boundsOf } from "../lib/groups";
  import { PX_PER_MM, mmToPx } from "../lib/units";

  let { pageEl, zoom }: { pageEl: HTMLElement | undefined; zoom: number } = $props();

  let moveable: Moveable | null = null;
  let movedId: string | null = null;
  let live: RectPx | null = null;

  const target = $derived.by(() => {
    const e = editor.selected;
    if (!e || e.locked || editor.cropping === e.id || editor.editingText === e.id) return null;
    const inGrid = editor.page.grid?.cells.some((c) => c.element_id === e.id);
    return inGrid ? null : e;
  });

  function start() {
    const e = target;
    if (!e) return;
    editor.gesture = true;
    live = {
      left: mmToPx(e.frame.x_mm, zoom),
      top: mmToPx(e.frame.y_mm, zoom),
      width: mmToPx(e.frame.w_mm, zoom),
      height: mmToPx(e.frame.h_mm, zoom),
      rotation: e.frame.rotation_deg,
    };
  }

  function commit(gesture: "move" | "resize" | "rotate", keptRatio = false) {
    editor.gesture = false;
    const e = target;
    if (!e || !live) return;
    let frame = rectPxToFrame(live, zoom);
    live = null;
    const f = e.frame;
    // Un simple clic ne bouge rien, même pas d'une retouche d'accrochage.
    if (frame.x_mm === f.x_mm && frame.y_mm === f.y_mm && frame.w_mm === f.w_mm && frame.h_mm === f.h_mm && frame.rotation_deg === f.rotation_deg) return;
    const lines = snapLines(
      editor.page.size,
      editor.page.elements.filter((o) => o.id !== e.id).map((o) => boundsOf([o.frame])),
    );
    const tol = 1 / (PX_PER_MM * zoom) + 0.05;
    if (gesture === "move") frame = settleMove(frame, lines, tol);
    // Proportions gardées (image, Maj) : pas de retouche bord par bord, qui déformerait.
    else if (gesture === "resize" && !keptRatio) frame = settleResize(f, frame, lines, tol);
    editor.updateElement(e.id, (el) => ({ ...el, frame }));
  }

  function create(el: HTMLElement): Moveable {
    const m = new Moveable(pageEl!, {
      target: el,
      draggable: true,
      resizable: true,
      rotatable: true,
      snappable: true,
      origin: false,
      throttleDrag: 0,
      throttleResize: 0,
      throttleRotate: 1,
      renderDirections: ["nw", "n", "ne", "w", "e", "sw", "s", "se"],
      isDisplaySnapDigit: false,
      snapThreshold: 6,
      // Le centre du bloc s'accroche aussi : milieux de la page et centres des autres blocs.
      snapDirections: { left: true, top: true, right: true, bottom: true, center: true, middle: true },
      elementSnapDirections: { left: true, top: true, right: true, bottom: true, center: true, middle: true },
      // Les images de la banque sont carrées : jamais déformées.
      keepRatio: target?.kind.type === "Image",
    });
    m.on("dragStart", start)
      .on("drag", (ev) => {
        if (!live) return;
        live.left = ev.left;
        live.top = ev.top;
        ev.target.style.left = `${ev.left}px`;
        ev.target.style.top = `${ev.top}px`;
      })
      .on("dragEnd", () => commit("move"))
      .on("resizeStart", (ev) => {
        start();
        ev.setOrigin(["%", "%"]);
        if (live) ev.dragStart?.set([live.left, live.top]);
      })
      .on("resize", (ev) => {
        if (!live) return;
        live.width = ev.width;
        live.height = ev.height;
        live.left = ev.drag.beforeTranslate[0];
        live.top = ev.drag.beforeTranslate[1];
        ev.target.style.width = `${ev.width}px`;
        ev.target.style.height = `${ev.height}px`;
        ev.target.style.left = `${live.left}px`;
        ev.target.style.top = `${live.top}px`;
      })
      .on("resizeEnd", () => commit("resize", !!moveable?.keepRatio))
      .on("rotateStart", (ev) => {
        start();
        if (live) ev.set(live.rotation);
      })
      .on("rotate", (ev) => {
        if (!live) return;
        live.rotation = ev.beforeRotate;
        ev.target.style.transform = ev.transform;
      })
      .on("rotateEnd", () => commit("rotate"));
    return m;
  }

  function onKey(ev: KeyboardEvent) {
    if (moveable) moveable.keepRatio = ev.shiftKey || target?.kind.type === "Image";
  }

  // Après chaque changement du document, du zoom ou de la sélection : recaler (ou recréer) les poignées.
  $effect(() => {
    void editor.doc;
    const z = zoom;
    const t = target;
    const container = pageEl;
    tick().then(() => {
      if (!container || live) return;
      const el = t ? container.querySelector<HTMLElement>(`[data-id="${t.id}"]`) : null;
      if (!el) {
        moveable?.destroy();
        moveable = null;
        movedId = null;
        return;
      }
      if (!moveable || movedId !== t!.id) {
        moveable?.destroy();
        moveable = create(el);
        movedId = t!.id;
      }
      const w = mmToPx(editor.page.size.width_mm, z);
      const h = mmToPx(editor.page.size.height_mm, z);
      const m = mmToPx(GUIDE_MARGIN_MM, z);
      moveable.snapGridWidth = mmToPx(SNAP_MM, z);
      moveable.snapGridHeight = mmToPx(SNAP_MM, z);
      moveable.elementGuidelines = [...container.querySelectorAll<HTMLElement>(".element")].filter((n) => n !== el);
      moveable.verticalGuidelines = [0, m, w / 2, w - m, w];
      moveable.horizontalGuidelines = [0, m, h / 2, h - m, h];
      moveable.updateRect();
    });
  });

  $effect(() => {
    window.addEventListener("keydown", onKey);
    window.addEventListener("keyup", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("keyup", onKey);
    };
  });

  onDestroy(() => moveable?.destroy());
</script>
