// Guides : comment se sert une fonction, en deux temps. Le livret se feuillette (une capture et deux ou trois
// phrases par panneau ; une courte vidéo seulement pour un geste, comme glisser d'une case à l'autre) ; « Me guider » montre ensuite, sur l'écran, le vrai bouton de chaque étape et
// passe à la suivante quand l'action est faite. Pour un nouveau guide : l'ajouter à GUIDES, ses captures dans
// `assets/guides/<id>/`.
import { editor } from "./docStore.svelte";
import { isPlacedCell } from "./table";
import { templates } from "./templates.svelte";

export interface GuidePanel {
  title: string;
  text: string;
  /** Capture (image fixe, ou animée pour un geste), servie depuis `assets/guides/<id>`. */
  image: string;
}

export interface WalkStep {
  /** Ce que la bulle montre (sélecteur CSS) ; null : bulle au milieu de l'écran. */
  target: string | null;
  title: string;
  text: string;
  /** L'étape est faite : on passe à la suivante. `base` est la valeur de `baseline` au début du pas à pas. */
  done?: (base: number) => boolean;
  baseline?: () => number;
}

export interface Guide {
  id: string;
  icon: string;
  title: string;
  summary: string;
  panels: GuidePanel[];
  walk: WalkStep[];
}

const img = (guide: string, name: string) => `/guides/${guide}/${name}.webp`;

export const GUIDES: Guide[] = [
  {
    id: "modele",
    icon: "▦",
    title: "Créer un modèle",
    summary: "Une page à réutiliser : ses cases à remplir à chaque fois, ses éléments fixes, enregistrée dans vos modèles.",
    panels: [
      {
        title: "Passer en mode modèle",
        text: "Bouton « Modèle », en haut à droite, ou Accueil > « Partir d'un modèle » > « Créer un modèle ». Un bandeau jaune rappelle que vous préparez un modèle.",
        image: img("modele", "mode"),
      },
      {
        title: "Poser une grille",
        text: "Le tiroir Grille découpe la page en cases. Glissez d'une case à une autre pour les fusionner : une grande case pour la photo principale, des petites autour.",
        image: img("modele", "grille"),
      },
      {
        title: "Photo ou texte",
        text: "Cliquez sur une case : sa barre choisit si l'on y déposera une photo ou si l'on y écrira. Chaque case est à remplir à chaque utilisation du modèle.",
        image: img("modele", "cases"),
      },
      {
        title: "À remplir ou Fixe",
        text: "Textes et images se posent aussi librement. Dans la barre de chacun : « À remplir » le vide dans le modèle (son texte devient l'invite), « Fixe » le garde tel quel, comme un titre ou le logo de l'école.",
        image: img("modele", "role"),
      },
      {
        title: "Ce qui n'est pas gardé",
        text: "Ce qui était déjà sur la page avant le mode modèle (une photo d'enfant, un texte écrit pour une sortie) n'est ni à remplir ni fixe : sa barre dit « pas gardé dans le modèle », et il n'y part pas.",
        image: img("modele", "garde"),
      },
      {
        title: "Enregistrer, puis réutiliser",
        text: "« Enregistrer comme modèle » dans le bandeau, donnez-lui un nom. Il attend ensuite dans le tiroir Modèles et dans « Partir d'un modèle », ses cases vides et prêtes.",
        image: img("modele", "enregistrer"),
      },
    ],
    walk: [
      {
        target: '[data-guide="template-mode"]',
        title: "Passez en mode modèle",
        text: "Cliquez sur « Modèle », en haut à droite.",
        done: () => editor.templateMode,
      },
      {
        target: ".drawer",
        title: "Posez une grille ou des éléments",
        text: "Dans le tiroir Grille, choisissez un découpage ; ou posez un texte, une image ou une forme depuis les autres tiroirs.",
        done: () => editor.templateMode && editor.page.elements.length > 0,
      },
      {
        target: ".ctx",
        title: "À remplir ou Fixe",
        text: "Les cases de la grille sont déjà à remplir. Cliquez sur un texte ou une image posés librement, puis choisissez « À remplir » ou « Fixe » dans sa barre. Sans élément libre, passez à l'étape suivante.",
        done: () => editor.page.elements.some((e) => !isPlacedCell(editor.page, e.id) && (e.zone || e.locked)),
      },
      {
        target: '[data-guide="save-template"]',
        title: "Enregistrez le modèle",
        text: "« Enregistrer comme modèle », puis donnez-lui un nom et validez.",
        baseline: () => templates.infos.length,
        done: (base) => templates.infos.length > base,
      },
      {
        target: '[data-guide="finish-template"]',
        title: "Terminez",
        text: "« Terminer le modèle » revient à la page ordinaire.",
        done: () => !editor.templateMode,
      },
      {
        target: '.rail [data-tour="templates"]',
        title: "Retrouvez-le",
        text: "Le tiroir Modèles le propose désormais, comme « Partir d'un modèle » sur l'accueil.",
        done: () => editor.drawer === "templates",
      },
    ],
  },
];

export const guideById = (id: string) => GUIDES.find((g) => g.id === id) ?? null;

/** Images d'un guide décodées à l'avance. Sans cela, WebKit décode une vidéo au fil de sa première lecture et,
 *  en retard, saute des images pour rattraper : elle semble accélérée la première fois. Les images gardées ici
 *  restent décodées. */
const decoded = new Map<string, HTMLImageElement>();

class Guides {
  /** Les images déjà décodées : le livret n'affiche une capture qu'une fois prête. */
  ready = $state<Record<string, true>>({});

  /** Décode les images des guides donnés (tous si aucun), une seule fois chacune. */
  preload(ids: string[] = GUIDES.map((g) => g.id)) {
    for (const g of GUIDES.filter((g) => ids.includes(g.id)))
      for (const { image } of g.panels) {
        if (decoded.has(image)) continue;
        const im = new Image();
        im.src = image;
        decoded.set(image, im);
        im.decode()
          .catch(() => {})
          .then(() => (this.ready = { ...this.ready, [image]: true }));
      }
  }

  /** Fenêtre des guides : la liste, ou le livret d'un guide. */
  showList = $state(false);
  open = $state<string | null>(null);
  /** Pas à pas en cours : le guide, son étape, et la valeur de départ de chaque étape. */
  walking = $state<{ id: string; step: number; bases: number[] } | null>(null);

  /** Ouvre la liste, ou directement le livret d'un guide. */
  show(id: string | null = null) {
    this.preload(id ? [id] : undefined);
    this.showList = true;
    this.open = id;
  }

  close() {
    this.showList = false;
    this.open = null;
  }

  /** Ferme le livret et lance le pas à pas. */
  walk(id: string) {
    this.close();
    const g = guideById(id);
    if (g) this.walking = { id, step: 0, bases: g.walk.map((s) => s.baseline?.() ?? 0) };
  }

  /** Étape suivante ; la dernière passée, le pas à pas s'arrête. */
  next() {
    const w = this.walking;
    const g = w && guideById(w.id);
    if (!w || !g) return;
    this.walking = w.step + 1 < g.walk.length ? { ...w, step: w.step + 1 } : null;
  }

  /** L'étape en cours est faite, ou celle d'après (faite sans passer par « Étape suivante ») : on avance. */
  advance() {
    const w = this.walking;
    const g = w && guideById(w.id);
    if (!w || !g) return;
    const isDone = (i: number) => i < g.walk.length && !!g.walk[i].done?.(w.bases[i]);
    if (isDone(w.step + 1)) {
      this.next();
      this.next();
    } else if (isDone(w.step)) this.next();
  }

  stop() {
    this.walking = null;
  }
}

export const guides = new Guides();
