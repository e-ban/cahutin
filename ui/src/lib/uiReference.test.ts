// Ce que l'interface trace, gardé dans crates/cahutin-mcp/src/fixtures/ui_reference.txt pour que l'assistant
// trace pareil. Avec UPDATE_REFERENCE=1, les lignes ROW_* du fichier sont réécrites (les autres restent).
import { readFileSync, writeFileSync } from "node:fs";
import { env } from "node:process";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import { graphismeRow } from "./graphisme";
import type { Element } from "./types";

const FILE = fileURLToPath(new URL("../../../crates/cahutin-mcp/src/fixtures/ui_reference.txt", import.meta.url));

/** Ce qui est comparé de chaque forme : sa sorte, son trait (style, couleur, épaisseur), son cadre, son tracé et la recette
 *  (les ids changent à chaque fois). */
function seen(els: Element[]) {
  return els.map((e) => {
    const s = e.kind.type === "Shape" ? e.kind : null;
    const stroke = s?.stroke ?? null;
    return { shape: s?.shape ?? null, style: stroke?.style ?? null, color: stroke?.color ?? null, width_mm: stroke?.width_mm ?? null, frame: e.frame, path: s?.path ?? null, graphisme: s?.graphisme ?? null };
  });
}

const ROWS: Record<string, Element[]> = {
  ROW_ponts: graphismeRow("ponts", "MS", 15, 25, 180),
  ROW_boucles: graphismeRow("boucles", "MS", 15, 25, 180),
  ROW_ronds: graphismeRow("ronds", "MS", 15, 25, 180),
  ROW_vagues_moitie_couloir: graphismeRow("vagues", "MS", 15, 25, 180, { preset: "moitie", guides: "Corridor" }),
  ROW_dents_gs_seul_aucune: graphismeRow("dents", "GS", 15, 25, 180, { preset: "seul", guides: "None", arrow: false }),
};

test("l'assistant a la même référence que l'interface", () => {
  const lines: string[] = readFileSync(FILE, "utf8").split("\n");
  if (env.UPDATE_REFERENCE === "1") {
    const kept = lines.filter((l) => !l.startsWith("ROW_") && l !== "");
    // Les lignes ROW_* à la place de la première, les autres clés à leur place.
    const at = Math.max(1, lines.findIndex((l) => l.startsWith("ROW_")));
    const rows = Object.entries(ROWS).map(([k, els]) => `${k}=${JSON.stringify(seen(els))}`);
    kept.splice(Math.min(at, kept.length), 0, ...rows);
    writeFileSync(FILE, kept.join("\n") + "\n");
  }
  const file: string[] = readFileSync(FILE, "utf8").split("\n");
  for (const [k, els] of Object.entries(ROWS)) {
    const line = file.find((l) => l.startsWith(`${k}=`));
    expect(line, k).toBeDefined();
    expect(JSON.parse(line!.slice(k.length + 1)), k).toEqual(seen(els));
  }
});
