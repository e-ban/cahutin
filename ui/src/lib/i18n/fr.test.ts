import { expect, test } from "vitest";
import { fr } from "./fr";

/** Toutes les chaînes de `fr`, fonctions comprises (appelées avec des valeurs d'exemple). */
function texts(value: unknown): string[] {
  if (typeof value === "string") return [value];
  if (typeof value === "function") return texts(value("x", "x", "x"));
  if (value && typeof value === "object") return Object.values(value).flatMap(texts);
  return [];
}

test("the app is called Cahutin", () => {
  expect(fr.appName).toBe("Cahutin");
  expect(texts(fr).filter((t) => t.includes("Carnet"))).toEqual([]);
});

test("versions before 1.0 say beta", () => {
  expect(fr.aboutVersion("0.5.0")).toContain("0.5.0 (bêta)");
  expect(fr.aboutVersion("1.0.0")).not.toContain("bêta");
});

/** Cahutin sert à toutes sortes de pages : l'accueil propose un modèle, le cahier de vie n'en est qu'un exemple. */
test("the welcome screen starts from a template, not only a life book", () => {
  expect(fr.welcomeTemplate).toBe("Partir d'un modèle");
  expect(fr.chooseTemplate).toBe("Choisissez un modèle");
});

/** Les photos retirées de la photothèque vont dans la corbeille de l'ordinateur, et la question le dit. */
test("removing class photos says they go to the computer's trash", () => {
  expect(fr.libraryTrash).toBe("Mettre à la corbeille");
  expect(fr.libraryDeleteConfirm(3)).toBe("Mettre ces 3 photos à la corbeille de l'ordinateur ? Vous pourrez les récupérer depuis la corbeille.");
  expect(fr.libraryDeleteConfirm(1)).toBe("Mettre cette photo à la corbeille de l'ordinateur ? Vous pourrez la récupérer depuis la corbeille.");
});

/** La question avant de quitter un document le nomme, et son retour ne s'appelle pas « Annuler ». */
test("the save question names the document and offers the way back", () => {
  expect(fr.saveChangesTo("Sortie à la ferme")).toBe("Enregistrer « Sortie à la ferme » avant de continuer ?");
  expect(fr.backToDocument).toBe("Revenir au document");
  expect(fr.dontSave).toBe("Ne pas enregistrer");
});
