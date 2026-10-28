import { expect, test } from "vitest";
import { summarizeImport } from "./importSummary";
import type { AssetMeta, ImportOutcome } from "./types";

const meta: AssetMeta = { id: "a", width_px: 10, height_px: 10, mime: "Jpeg" };

test("summarizeImport all ok", () => {
  const r = summarizeImport([{ status: "Ok", path: "/p/a.jpg", folder: "p", asset: meta }]);
  expect(r.imported).toEqual([{ meta, path: "/p/a.jpg", folder: "p" }]);
  expect(r.message).toBeNull();
});

test("summarizeImport lists failures by file name", () => {
  const outcomes: ImportOutcome[] = [
    { status: "Ok", path: "C:\\Photos\\a.jpg", folder: "Photos", asset: meta },
    { status: "Err", path: "C:\\Photos\\IMG_1.HEIC", message: "format HEIC (iPhone) non pris en charge : enregistrez la photo en JPEG" },
    { status: "Err", path: "/home/élodie/abîmé.jpg", message: "fichier illisible ou abîmé" },
  ];
  const r = summarizeImport(outcomes);
  expect(r.imported).toEqual([{ meta, path: "C:\\Photos\\a.jpg", folder: "Photos" }]);
  expect(r.message).toBe(
    "2 photos n'ont pas pu être importées :\nIMG_1.HEIC : format HEIC (iPhone) non pris en charge : enregistrez la photo en JPEG\nabîmé.jpg : fichier illisible ou abîmé",
  );
  expect(r.message).not.toContain("Photos\\");
});

test("summarizeImport singular", () => {
  const r = summarizeImport([{ status: "Err", path: "x.heic", message: "m" }]);
  expect(r.message).toBe("1 photo n'a pas pu être importée :\nx.heic : m");
});

test("an empty folder says so", () => {
  expect(summarizeImport([], { fromFolder: true }).message).toBe("Aucune photo trouvée dans ce dossier (JPEG, PNG ou WebP attendus).");
  expect(summarizeImport([]).message).toBeNull();
});
