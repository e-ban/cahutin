// Conversion entre le contenu TipTap et le texte riche du document. Seule la mise en forme que l'export
// ODG sait représenter survit : tout le reste (titres, listes numérotées, liens, surlignage...) devient du texte
// simple. La puce d'un paragraphe est son attribut `list` ; une liste à puces venue d'ailleurs donne des
// paragraphes à puce, à un seul niveau.
import type { JSONContent } from "@tiptap/core";
import { splitFields } from "./classFields";
import { isKnownFamily } from "./fontNames";
import type { Align, Paragraph, RichText, Run, TextStyle } from "./types";

/** Marge intérieure des zones de texte sans réglage, identique à l'export ODG (carnet_core::TEXT_PADDING_MM). */
export const TEXT_PADDING_MM = 1.5;

/** Marge intérieure d'un texte : son réglage, sinon la marge par défaut. */
export const textPadding = (style: TextStyle) => style.padding_mm ?? TEXT_PADDING_MM;

const ALIGN_TO_CSS: Record<Align, string> = { Left: "left", Center: "center", Right: "right", Justify: "justify" };
const CSS_TO_ALIGN: Record<string, Align> = { left: "Left", center: "Center", right: "Right", justify: "Justify" };

export const alignToCss = (a: Align) => ALIGN_TO_CSS[a];

/**
 * Texte brut collé ou déposé : les retours à la ligne manuels de Word (U+000B, U+000C) et les fins de ligne
 * Windows deviennent des retours à la ligne ; les autres caractères de contrôle, interdits en XML, disparaissent.
 */
export function sanitizePlainText(text: string): string {
  return text
    .replace(/\r\n?|[\u000b\u000c]/g, "\n")
    .replace(/[\u0000-\u0008\u000e-\u001f\u007f]/g, "");
}

const HEX_COLOR = /^#[0-9a-fA-F]{6}$/;

export function parsePt(size: unknown): number | null {
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
      // Seules les valeurs que l'export ODG représente fidèlement sont gardées.
      if (typeof a.color === "string" && HEX_COLOR.test(a.color) && a.color.toLowerCase() !== style.color.toLowerCase()) run.color = a.color;
      if (typeof a.fontFamily === "string" && isKnownFamily(a.fontFamily) && a.fontFamily !== style.font) run.font = a.fontFamily;
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

function paragraphFrom(node: JSONContent, style: TextStyle, bullet: boolean): Paragraph {
  const runs: Run[] = [];
  for (const child of node.content ?? []) {
    if (child.type === "text") pushRun(runs, runFrom(child, style));
    // Un champ de la classe (pastille) redevient ses accolades, avec la mise en forme de la pastille.
    else if (child.type === "classField") pushRun(runs, { ...runFrom({ text: "", marks: child.marks }, style), text: `{${child.attrs?.key}}` });
    // Un retour à la ligne garde la mise en forme de son passage : le texte se rouvre et s'enregistre à l'identique.
    else if (child.type === "hardBreak") pushRun(runs, { ...runFrom({ text: "", marks: child.marks }, style), text: "\n" });
  }
  const css = node.attrs?.textAlign;
  const align = typeof css === "string" ? (CSS_TO_ALIGN[css] ?? null) : null;
  const p: Paragraph = { align: align === style.align ? null : align, runs };
  // Sans puce, pas de clé : le texte se compare et s'enregistre comme avant.
  if (bullet || node.attrs?.list === "Bullet") p.list = "Bullet";
  return p;
}

/** Un bloc qui contient du texte devient un paragraphe ; les autres blocs (listes...) sont parcourus. Dans une
 *  liste à puces (même au fond d'une liste numérotée), chaque paragraphe porte une puce. */
function collect(node: JSONContent, style: TextStyle, out: Paragraph[], bullet = false) {
  const children = node.content ?? [];
  const isTextBlock = children.some((c) => c.type === "text" || c.type === "hardBreak") || node.type === "paragraph" || node.type === "heading";
  if (isTextBlock && node.type !== "doc") out.push(paragraphFrom(node, style, bullet));
  else for (const c of children) collect(c, style, out, bullet || node.type === "bulletList");
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
      const node: JSONContent = { type: "paragraph", attrs: { textAlign: p.align ? ALIGN_TO_CSS[p.align] : null, ...(p.list ? { list: p.list } : {}) } };
      const content: JSONContent[] = [];
      for (const r of p.runs) {
        const marks: JSONContent["marks"] = [];
        if (r.bold) marks.push({ type: "bold" });
        if (r.italic) marks.push({ type: "italic" });
        if (r.underline) marks.push({ type: "underline" });
        if (r.color || r.font || r.size_pt) {
          marks.push({ type: "textStyle", attrs: { color: r.color, fontFamily: r.font, fontSize: r.size_pt ? `${r.size_pt}pt` : null } });
        }
        r.text.split("\n").forEach((piece, i) => {
          if (i > 0) content.push(marks.length ? { type: "hardBreak", marks } : { type: "hardBreak" });
          if (!piece) return;
          // Chaque champ de la classe devient une pastille, un seul caractère pour l'éditeur.
          for (const part of splitFields(piece)) {
            if ("field" in part) content.push(marks.length ? { type: "classField", attrs: { key: part.field }, marks } : { type: "classField", attrs: { key: part.field } });
            else content.push(marks.length ? { type: "text", text: part.text, marks } : { type: "text", text: part.text });
          }
        });
      }
      if (content.length) node.content = content;
      return node;
    }),
  };
}

/** Nom de police pour le CSS, entre guillemets : sans eux, un nom avec un chiffre seul, un point ou une
 *  parenthèse (« Cursive 2 ») est ignoré et le texte prend la police de sa zone. */
export function cssFontFamily(name: string): string {
  return `"${name.replace(/["\\]/g, "\\$&")}"`;
}

/** Le nom de police d'une valeur CSS `font-family` (le premier), avec ou sans guillemets. */
export function fontFamilyName(css: string): string | null {
  const first = css.trim().match(/^"((?:[^"\\]|\\.)*)"|^'((?:[^'\\]|\\.)*)'|^([^,]+)/);
  const name = (first?.[1] ?? first?.[2] ?? first?.[3] ?? "").replace(/\\(.)/g, "$1").trim();
  return name || null;
}
