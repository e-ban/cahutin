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

/** « Retirer » ne touche à aucun fichier : il masque la photo dans la photothèque, et le message le dit. */
test("hiding class photos says the files stay on the computer", () => {
  expect(fr.libraryHide).toBe("Retirer");
  expect(fr.libraryHidden(1)).toBe("1 photo retirée de la photothèque. Le fichier reste sur l'ordinateur.");
  expect(fr.libraryHidden(3)).toBe("3 photos retirées de la photothèque. Les fichiers restent sur l'ordinateur.");
});

/** Dossiers suivis : la recherche au fil de l'eau, l'oubli d'un dossier, un dossier introuvable. */
test("followed folders speak plainly, singular and plural", () => {
  expect(fr.libraryScanning(0)).toBe("Recherche des photos…");
  expect(fr.libraryScanning(1)).toBe("Recherche des photos : 1 trouvée…");
  expect(fr.libraryScanning(42)).toBe("Recherche des photos : 42 trouvées…");
  expect(fr.libraryUnfollowConfirm("Sortie")).toBe(
    "Ne plus suivre « Sortie » ? Ses photos ne seront plus montrées ici, mais elles restent sur l'ordinateur : aucun fichier n'est touché. Les élèves et activités indiqués pour elles seront oubliés.",
  );
  expect(fr.libraryUnfollowed("Sortie")).toBe("« Sortie » n'est plus suivi. Ses photos restent sur l'ordinateur.");
  expect(fr.libraryFolderMissing("Clé USB")).toContain("« Clé USB »");
  expect(fr.libraryEmpty).toContain("« Ajouter un dossier »");
  expect(fr.libraryEmpty).not.toContain("Ajouter des photos");
});

/** Migration : les copies d'une version précédente, rapprochées de leurs originaux. */
test("the migration banner and its result", () => {
  expect(fr.libraryMigration).toBe(
    "Vos photos ont été copiées dans Cahutin par une version précédente. Indiquez les dossiers où sont vos originaux, puis « Retrouver les originaux » : Cahutin y reportera les enfants et activités.",
  );
  expect(fr.libraryFindOriginals).toBe("Retrouver les originaux");
  expect(fr.libraryLater).toBe("Plus tard");
  // Refuser la migration : la question ne revient plus, et l'on dit où retrouver l'action.
  expect(fr.libraryNoThanks).toBe("Non merci");
  expect(fr.libraryMigrationDeclined).toBe(
    "Entendu, cette question ne reviendra plus. Vos photos restent comme elles sont. Le lien « Retrouver les originaux » reste en bas de la colonne de gauche.",
  );
  expect(fr.libraryOriginalsFound(12, 3)).toBe(
    "12 photos retrouvées, 3 non retrouvées. Mettre les 12 copies à la corbeille ? Les enfants et activités sont reportés sur les originaux.",
  );
  expect(fr.libraryOriginalsFound(1, 1)).toBe(
    "1 photo retrouvée, 1 non retrouvée. Mettre la copie à la corbeille ? Les enfants et activités sont reportés sur les originaux.",
  );
  // Rien de manquant : on ne parle pas de « 0 non retrouvée ».
  expect(fr.libraryOriginalsFound(2, 0)).toBe(
    "2 photos retrouvées. Mettre les 2 copies à la corbeille ? Les enfants et activités sont reportés sur les originaux.",
  );
  expect(fr.libraryOriginalsFound(1, 0)).toBe(
    "1 photo retrouvée. Mettre la copie à la corbeille ? Les enfants et activités sont reportés sur les originaux.",
  );
  expect(fr.libraryOriginalsFound(3, 2)).toBe(
    "3 photos retrouvées, 2 non retrouvées. Mettre les 3 copies à la corbeille ? Les enfants et activités sont reportés sur les originaux.",
  );
  // Le bandeau ne revient plus : on dit où est le lien.
  expect(fr.libraryOriginalsNone).toBe(
    "Aucun original retrouvé dans les dossiers suivis : rien n'a changé. Ajoutez le dossier où sont vos originaux, puis cliquez sur « Retrouver les originaux », dans la colonne de gauche.",
  );
  expect(fr.libraryOriginalsDone(1)).toBe("1 copie mise à la corbeille de l'ordinateur. Les enfants et activités sont sur l'original.");
  expect(fr.libraryOriginalsDone(4)).toBe("4 copies mises à la corbeille de l'ordinateur. Les enfants et activités sont sur les originaux.");
  // La corbeille en refuse une partie : on dit combien, et que rien n'est perdu.
  expect(fr.libraryOriginalsNotTrashed(3, 2)).toBe(
    "3 copies mises à la corbeille de l'ordinateur, leurs enfants et activités sont sur les originaux. 2 copies n'ont pas pu être mises à la corbeille : elles restent dans la photothèque, avec leurs enfants et activités. Vous pourrez réessayer avec « Retrouver les originaux ».",
  );
  expect(fr.libraryOriginalsNotTrashed(0, 1)).toBe(
    "1 copie n'a pas pu être mise à la corbeille : elle reste dans la photothèque, avec ses enfants et activités. Vous pourrez réessayer avec « Retrouver les originaux ».",
  );
  expect(fr.libraryOriginalsNotTrashed(1, 1)).toContain("1 copie mise à la corbeille de l'ordinateur, ses enfants et activités sont sur l'original.");
});

/** La question avant de quitter un document le nomme, et son retour ne s'appelle pas « Annuler ». */
test("the save question names the document and offers the way back", () => {
  expect(fr.saveChangesTo("Sortie à la ferme")).toBe("Enregistrer « Sortie à la ferme » avant de continuer ?");
  expect(fr.backToDocument).toBe("Revenir au document");
  expect(fr.dontSave).toBe("Ne pas enregistrer");
});

/** Signaler un problème : la messagerie s'ouvre, le message dit quel fichier joindre et à qui écrire sinon. */
test("the problem report says which file to attach and where to write", () => {
  const opened = fr.reportMailOpened("rapport-carnet.txt", "Documents", "aide@exemple.fr");
  expect(opened).toContain("« rapport-carnet.txt »");
  expect(opened).toContain("Documents");
  expect(opened).toContain("aide@exemple.fr");
  expect(fr.reportMailFailed("rapport-carnet.txt", "aide@exemple.fr")).toContain("aide@exemple.fr");
});

/** La barre de proposition : « 2 modifications » puis une phrase qui nomme la proposition, sans accord bancal. */
test("the proposal bar reads well whatever the count", () => {
  expect(fr.assistant.locked).toBe("Acceptez ou refusez cette proposition avant de modifier la page (Ctrl+Z la refuse).");
});
