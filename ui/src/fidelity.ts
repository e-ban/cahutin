// Page de développement : rend chaque cas de fixtures/text_fidelity.json exactement comme une zone de
// texte de l'éditeur, compte les lignes et affiche {id: lignes} dans #result.
import { mount } from "svelte";
import "./fonts.css";
import cases from "../../fixtures/text_fidelity.json";
import TextEl from "./components/elements/TextEl.svelte";
import type { Element } from "./lib/types";

const root = document.getElementById("cases")!;

function countLines(box: HTMLElement): number {
  const tops = new Set<number>();
  const walker = document.createTreeWalker(box, NodeFilter.SHOW_TEXT);
  while (walker.nextNode()) {
    const range = document.createRange();
    range.selectNodeContents(walker.currentNode);
    for (const r of range.getClientRects()) if (r.width > 0) tops.add(Math.round(r.top));
  }
  const sorted = [...tops].sort((a, b) => a - b);
  return sorted.filter((t, i) => i === 0 || t - sorted[i - 1] > 2).length;
}

for (const c of cases) {
  const wrap = document.createElement("div");
  wrap.style.cssText = `position:relative;margin:20px;width:${c.w_mm}mm;height:${c.h_mm}mm;outline:1px dotted #ccc`;
  wrap.dataset.id = c.id;
  root.appendChild(wrap);
  const element: Element = {
    id: c.id,
    frame: { x_mm: 0, y_mm: 0, w_mm: c.w_mm, h_mm: c.h_mm, rotation_deg: 0 },
    locked: false,
    zone: false,
    kind: {
      type: "Text",
      content: { paragraphs: [{ align: null, runs: [{ text: c.text, bold: false, italic: false, underline: false, color: null, font: null, size_pt: null }] }] },
      style: { font: c.font, size_pt: c.size_pt, color: "#222222", align: "Left" },
      prompt: null,
    },
  };
  if (element.kind.type === "Text") mount(TextEl, { target: wrap, props: { element, props: element.kind, zoom: 1 } });
}

document.fonts.ready.then(() =>
  requestAnimationFrame(() => {
    const result: Record<string, number> = {};
    for (const wrap of root.querySelectorAll<HTMLElement>("[data-id]")) result[wrap.dataset.id!] = countLines(wrap.querySelector(".text")!);
    document.getElementById("result")!.textContent = JSON.stringify(result);
  }),
);
