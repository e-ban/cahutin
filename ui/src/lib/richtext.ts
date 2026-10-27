// Conversion entre le contenu TipTap et le texte riche du document. Seule la mise en forme que l'export
// ODG sait représenter survit : tout le reste (titres, listes, liens, surlignage...) devient du texte simple.
import type { JSONContent } from "@tiptap/core";
import type { Align, Paragraph, RichText, Run, TextStyle } from "./types";

/** Marge intérieure des zones de texte, identique à l'export ODG (carnet_core::odg::TEXT_PADDING_MM). */
export const TEXT_PADDING_MM = 1.5;

const ALIGN_TO_CSS: Record<Align, string> = { Left: "left", Center: "center", Right: "right", Justify: "justify" };
const CSS_TO_ALIGN: Record<string, Align> = { left: "Left", center: "Center", right: "Right", justify: "Justify" };

export const alignToCss = (a: Align) => ALIGN_TO_CSS[a];

function parsePt(size: unknown): number | null {
  if (typeof size !== "string") return null;
  const m = size.match(/^([\d.]+)pt$/);
  return m ? Number(m[1]) : null;
}

function runFrom(node: JSONContent, style: TextStyle): Run {
  const run: Run = { text: node.text ?? "", bold: false, italic: false, underline: false, color: null, font: null, size_pt: null };
  for (const mark of node.marks ?? []) {
    if (mark.type === "bold") run.bold = true;
    else if (mark.type === "italic") run.italic = true;
    else if (mark.type === "underline") run.underline = true;
    else if (mark.type === "textStyle") {
      const a = mark.attrs ?? {};
      if (typeof a.color === "string" && a.color && a.color.toLowerCase() !== style.color.toLowerCase()) run.color = a.color;
      if (typeof a.fontFamily === "string" && a.fontFamily && a.fontFamily !== style.font) run.font = a.fontFamily;
      const pt = parsePt(a.fontSize);
      if (pt !== null && pt !== style.size_pt) run.size_pt = pt;
    }
  }
  return run;
}

const sameFormat = (a: Run, b: Run) =>
  a.bold === b.bold && a.italic === b.italic && a.underline === b.underline && a.color === b.color && a.font === b.font && a.size_pt === b.size_pt;

function pushRun(runs: Run[], run: Run) {
  if (!run.text) return;
  const last = runs.at(-1);
  if (last && sameFormat(last, run)) last.text += run.text;
  else runs.push(run);
}

function paragraphFrom(node: JSONContent, style: TextStyle): Paragraph {
  const runs: Run[] = [];
  for (const child of node.content ?? []) {
    if (child.type === "text") pushRun(runs, runFrom(child, style));
    else if (child.type === "hardBreak") pushRun(runs, { ...(runs.at(-1) ?? runFrom({ text: "" }, style)), text: "\n" });
  }
  const css = node.attrs?.textAlign;
  const align = typeof css === "string" ? (CSS_TO_ALIGN[css] ?? null) : null;
  return { align: align === style.align ? null : align, runs };
}

/** Un bloc qui contient du texte devient un paragraphe ; les autres blocs (listes...) sont parcourus. */
function collect(node: JSONContent, style: TextStyle, out: Paragraph[]) {
  const children = node.content ?? [];
  const isTextBlock = children.some((c) => c.type === "text" || c.type === "hardBreak") || node.type === "paragraph" || node.type === "heading";
  if (isTextBlock && node.type !== "doc") out.push(paragraphFrom(node, style));
  else for (const c of children) collect(c, style, out);
}

export function tiptapToRich(doc: JSONContent, style: TextStyle): RichText {
  const paragraphs: Paragraph[] = [];
  collect(doc, style, paragraphs);
  return { paragraphs };
}

export function richToTiptap(rt: RichText): JSONContent {
  return {
    type: "doc",
    content: rt.paragraphs.map((p) => {
      const node: JSONContent = { type: "paragraph", attrs: { textAlign: p.align ? ALIGN_TO_CSS[p.align] : null } };
      const content: JSONContent[] = [];
      for (const r of p.runs) {
        r.text.split("\n").forEach((piece, i) => {
          if (i > 0) content.push({ type: "hardBreak" });
          if (!piece) return;
          const marks: JSONContent["marks"] = [];
          if (r.bold) marks.push({ type: "bold" });
          if (r.italic) marks.push({ type: "italic" });
          if (r.underline) marks.push({ type: "underline" });
          if (r.color || r.font || r.size_pt) {
            marks.push({ type: "textStyle", attrs: { color: r.color, fontFamily: r.font, fontSize: r.size_pt ? `${r.size_pt}pt` : null } });
          }
          content.push(marks.length ? { type: "text", text: piece, marks } : { type: "text", text: piece });
        });
      }
      if (content.length) node.content = content;
      return node;
    }),
  };
}
