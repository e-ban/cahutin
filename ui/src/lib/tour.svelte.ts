// Visite guidée : 7 étapes, une zone de l'écran mise en lumière et une bulle d'explication. Montrée au
// premier lancement, à la fermeture de l'écran d'accueil ; relançable depuis le menu Aide.
import { readPref, writePref } from "./prefs";

export interface TourStep {
  /** Zone mise en lumière (sélecteur CSS) ; aucune : bulle seule, au centre. */
  target: string | null;
  title: string;
  text: string;
}

export const TOUR_STEPS: TourStep[] = [
  { target: null, title: "Bienvenue dans Carnet", text: "En une minute, voici où trouver chaque chose. Vous pourrez revoir cette visite depuis le menu Aide." },
  { target: ".rail", title: "Les outils", text: "Modèles, photos, textes, formes, polices et décors : tout ce que l'on ajoute à la page se trouve dans cette colonne." },
  { target: "main.stage .page", title: "La page", text: "Glissez une photo sur une zone pour la placer. Double-cliquez sur un texte pour écrire." },
  {
    target: "main.stage .page",
    title: "Modifier un élément",
    text: "Cliquez sur un élément : une barre apparaît au-dessus pour changer sa police, sa couleur ou son cadre. Le clic droit propose copier, dupliquer, mettre devant.",
  },
  { target: ".right", title: "Les réglages", text: "Position, taille, marges et alignement au millimètre près, pour l'élément choisi." },
  { target: ".pagebar", title: "Les pages", text: "Ajoutez, dupliquez ou supprimez une page, changez son fond ou passez-la en paysage." },
  {
    target: ".top",
    title: "Enregistrer et imprimer",
    text: "Le menu Fichier enregistre, ouvre et importe vos documents. « Imprimer / Exporter » imprime, crée un PDF ou un fichier pour LibreOffice.",
  },
];

export interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** Écart entre la bulle et la zone, et marge minimale avec les bords de la fenêtre. */
const GAP = 12;
const MARGIN = 12;

/** Bulle à droite de la zone, sinon dessous, sinon dessus, sinon dedans ; toujours entière dans la fenêtre. */
export function placeBubble(target: Box | null, bubble: { width: number; height: number }, viewport: { width: number; height: number }): { left: number; top: number } {
  const clampX = (x: number) => Math.max(MARGIN, Math.min(x, viewport.width - MARGIN - bubble.width));
  const clampY = (y: number) => Math.max(MARGIN, Math.min(y, viewport.height - MARGIN - bubble.height));
  if (!target) return { left: (viewport.width - bubble.width) / 2, top: (viewport.height - bubble.height) / 2 };
  const fitsX = (x: number) => x >= MARGIN && x + bubble.width <= viewport.width - MARGIN;
  const fitsY = (y: number) => y >= MARGIN && y + bubble.height <= viewport.height - MARGIN;
  const right = target.left + target.width + GAP;
  if (fitsX(right)) return { left: right, top: clampY(target.top) };
  const below = target.top + target.height + GAP;
  if (fitsY(below)) return { left: clampX(target.left), top: below };
  const above = target.top - GAP - bubble.height;
  if (fitsY(above)) return { left: clampX(target.left), top: above };
  return { left: clampX(target.left + GAP), top: clampY(target.top + GAP) };
}

/** La visite démarre quand l'écran d'accueil se ferme, si elle n'a encore jamais été faite ni passée. */
export function shouldStartTour(done: boolean, welcomeWasOpen: boolean, welcomeOpen: boolean): boolean {
  return !done && welcomeWasOpen && !welcomeOpen;
}

class Tour {
  /** Étape affichée ; null : pas de visite en cours. */
  step = $state<number | null>(null);
  done = $state(readPref("tourDone", false));

  start() {
    this.step = 0;
  }

  next() {
    if (this.step === null) return;
    if (this.step >= TOUR_STEPS.length - 1) this.end();
    else this.step++;
  }

  prev() {
    if (this.step !== null && this.step > 0) this.step--;
  }

  /** Fin ou « Passer » : la visite ne revient plus d'elle-même. */
  end() {
    this.step = null;
    this.done = true;
    writePref("tourDone", true);
  }
}

export const tour = new Tour();
