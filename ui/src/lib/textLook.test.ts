import { expect, test } from "vitest";
import { finishedText, liftUniformLook, lookOnWholeText } from "./textLook";
import type { TextProps } from "./types";

const run = (text: string, own: { font?: string; size_pt?: number; color?: string } = {}) => ({
  text,
  bold: true,
  italic: false,
  underline: false,
  color: own.color ?? null,
  font: own.font ?? null,
  size_pt: own.size_pt ?? null,
});
const props = (): TextProps => ({
  content: { paragraphs: [{ align: "Right", runs: [run("Bonjour "), run("Léa", { font: "Arial", size_pt: 30, color: "#ff0000" })] }] },
  style: { font: "Andika", size_pt: 14, color: "#000000", align: "Left", named: "texte" },
  prompt: null,
});

test("une taille donnée au bloc entier s'applique aussi aux mots qui avaient la leur", () => {
  const t = lookOnWholeText(props(), { size_pt: 96 });
  expect(t.style.size_pt).toBe(96);
  expect(t.content.paragraphs[0].runs.map((r) => r.size_pt)).toEqual([null, null]);
  // Le reste de la mise en forme des mots ne bouge pas.
  expect(t.content.paragraphs[0].runs[1]).toMatchObject({ font: "Arial", color: "#ff0000", bold: true, text: "Léa" });
  expect(t.content.paragraphs[0].align).toBe("Right");
  expect(t.style.named).toBe("texte");
});

test("une police ou une couleur donnée au bloc entier s'applique à tous les mots", () => {
  const f = lookOnWholeText(props(), { font: "OpenDyslexic" });
  expect(f.style.font).toBe("OpenDyslexic");
  expect(f.content.paragraphs[0].runs[1]).toMatchObject({ font: null, size_pt: 30, color: "#ff0000" });
  const c = lookOnWholeText(props(), { color: "#1f2c4a" });
  expect(c.style.color).toBe("#1f2c4a");
  expect(c.content.paragraphs[0].runs[1]).toMatchObject({ font: "Arial", size_pt: 30, color: null });
});

test("à la fin de l'écriture, une taille portée par tous les passages devient celle du bloc", () => {
  const t = props();
  t.content.paragraphs = [
    { align: null, runs: [run("Bonjour ", { size_pt: 30 }), run("Léa", { font: "Arial", size_pt: 30, color: "#ff0000" })] },
    { align: null, runs: [run("\n", { size_pt: 30 })] },
  ];
  const lifted = liftUniformLook(t);
  expect(lifted.style).toMatchObject({ size_pt: 30, font: "Andika", color: "#000000", named: "texte" });
  expect(lifted.content.paragraphs.flatMap((p) => p.runs).map((r) => r.size_pt)).toEqual([null, null, null]);
  // Police et couleur, portées par un seul passage, restent sur lui.
  expect(lifted.content.paragraphs[0].runs[1]).toMatchObject({ font: "Arial", color: "#ff0000" });
});

test("police et couleur communes à tous les passages remontent aussi au bloc", () => {
  const t = props();
  t.content.paragraphs = [{ align: null, runs: [run("Bonjour ", { font: "Arial", color: "#ff0000" }), run("Léa", { font: "Arial", color: "#ff0000", size_pt: 30 })] }];
  const lifted = liftUniformLook(t);
  expect(lifted.style).toMatchObject({ font: "Arial", color: "#ff0000", size_pt: 14 });
  expect(lifted.content.paragraphs[0].runs.map((r) => [r.font, r.color, r.size_pt])).toEqual([[null, null, null], [null, null, 30]]);
});

test("sans valeur commune à tous les passages, ni texte, rien ne change", () => {
  const t = props();
  expect(liftUniformLook(t)).toBe(t);
  const empty: TextProps = { ...t, content: { paragraphs: [{ align: null, runs: [] }] } };
  expect(liftUniformLook(empty)).toBe(empty);
});

test("un texte rouvert sans changement reste tel quel, même si tous ses passages ont leur taille", () => {
  const t = props();
  t.content.paragraphs = [{ align: null, runs: [run("Bonjour", { size_pt: 30 })] }];
  expect(finishedText(t, structuredClone(t.content))).toBeNull();
  const typed = { paragraphs: [{ align: null, runs: [run("Bonjour !", { size_pt: 30 })] }] };
  expect(finishedText(t, typed)).toMatchObject({ style: { size_pt: 30 }, content: { paragraphs: [{ runs: [{ text: "Bonjour !", size_pt: null }] }] } });
});
