// Nouveautés des versions : une fenêtre montrée une fois, après la mise à jour, à qui utilisait déjà
// Cahutin (un nouvel utilisateur a la visite guidée). Rouvrable depuis le menu Aide, où l'on retrouve aussi
// les nouveautés des versions précédentes. Pour une nouvelle version : l'ajouter en tête de RELEASES, ses
// captures dans `assets/nouveautes/<version>/`.

export interface Novelty {
  title: string;
  text: string;
  /** Capture d'écran, servie depuis `assets/nouveautes/<version>`. */
  image: string;
}

export interface Release {
  version: string;
  items: Novelty[];
}

/** De la plus récente à la plus ancienne. */
export const RELEASES: Release[] = [
  {
    version: "0.6",
    items: [
      {
        title: "Votre classe",
        text: "Sur l'accueil, « Ma classe » : collez la liste des prénoms, c'est tout. Si vous le souhaitez, ajoutez les photos d'un coup et cochez l'autorisation des familles, enfant par enfant. La classe reste sur votre ordinateur.",
        image: "/nouveautes/0.6/classe.webp",
      },
      {
        title: "Des étiquettes remplies en un clic",
        text: "Sur une page qui attend des prénoms, le bandeau bleu la remplit avec votre classe : un enfant par étiquette, dans toutes ses écritures. Essayez le nouveau modèle « Étiquettes 3 écritures ».",
        image: "/nouveautes/0.6/etiquettes.webp",
      },
      {
        title: "Flouter des visages",
        text: "Pour les enfants sans autorisation photo : sélectionnez une photo, puis « Flouter ». Cliquez sur les visages à cacher, ou tracez une zone à la main. Le repérage automatique se fait sur l'ordinateur, avec votre accord.",
        image: "/nouveautes/0.6/flou.webp",
      },
      {
        title: "Des tableaux",
        text: "Tiroir Texte, « Ajouter un tableau » : emploi du temps, tableau de présence, loto. Une case reçoit un texte, un pictogramme ou une photo ; on fusionne les cases et on ajoute des lignes d'un clic.",
        image: "/nouveautes/0.6/tableau.webp",
      },
      {
        title: "Repères de découpe et lettres à repasser",
        text: "« Découpe » trace des traits aux coins ou des pointillés autour des étiquettes. Dans le tiroir Texte, les styles « À repasser » et « Lettres creuses » préparent l'écriture du prénom.",
        image: "/nouveautes/0.6/decoupe.webp",
      },
    ],
  },
];

/** La version la plus récente, celle que la fenêtre montre d'abord. */
export const WHATS_NEW = RELEASES[0];

/** À montrer une fois par version, à qui avait déjà fait (ou passé) la visite guidée. */
export function shouldShowWhatsNew(seen: string | null, returningUser: boolean): boolean {
  return returningUser && seen !== WHATS_NEW.version;
}
