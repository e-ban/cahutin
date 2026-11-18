// Le crayon (dessin à main levée) : actif, chaque tracé sur la page devient un chemin (FreehandLayer).
class Pen {
  /** Le crayon est pris : la page dessine au lieu de sélectionner. */
  active = $state(false);
}

export const pen = new Pen();
