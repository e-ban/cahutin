import type { AssetMeta, ImportOutcome } from "./types";

const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;

/** Photos importées, et un message listant les échecs par nom de fichier (sans le dossier). */
export function summarizeImport(outcomes: ImportOutcome[]): { imported: AssetMeta[]; message: string | null } {
  const imported: AssetMeta[] = [];
  const failures: string[] = [];
  for (const o of outcomes) {
    if (o.status === "Ok") imported.push(o.asset);
    else failures.push(`${fileName(o.path)} : ${o.message}`);
  }
  if (failures.length === 0) return { imported, message: null };
  const head =
    failures.length === 1 ? "1 photo n'a pas pu être importée :" : `${failures.length} photos n'ont pas pu être importées :`;
  return { imported, message: [head, ...failures].join("\n") };
}
