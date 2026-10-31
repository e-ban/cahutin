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
    version: "0.8",
    items: [
      {
        title: "Des modèles pour la classe",
        text: "« Partir d'un modèle » > « Pour la classe » : memory et loto des prénoms, lettres mobiles, J'écris mon prénom, tableau des présences, affiche des anniversaires. Remplissez-les d'un clic avec votre classe.",
        image: "/nouveautes/0.8/modeles-classe.webp",
      },
      {
        title: "Lignes d'écriture et styles de texte",
        text: "Tiroir Texte : des lignes Seyès, simples ou à petits carreaux, pour écrire sous un modèle à repasser. Le bouton « Style » donne l'allure Titre, Consigne ou Prénom, et la change partout d'un coup.",
        image: "/nouveautes/0.8/lignes.webp",
      },
      {
        title: "Répéter un bloc, formats A5 et A3",
        text: "Sélectionnez une étiquette, puis « Répéter » : la page se remplit d'exemplaires prêts pour la classe. La page passe en A5 ou A3 avec son contenu, et une affiche A3 s'imprime sur deux A4.",
        image: "/nouveautes/0.8/repeter.webp",
      },
      {
        title: "Votre classe, plus complète",
        text: "Dates de naissance, champs {âge} et {anniversaire}, et la liste importée d'un tableur ou d'un export ONDE.",
        image: "/nouveautes/0.8/classe.webp",
      },
      {
        title: "Onze exemples",
        text: "Emploi du temps, loto des émotions, règles de la classe en A3, cinq sens, porte-manteaux : des pages prêtes à modifier, depuis l'accueil.",
        image: "/nouveautes/0.8/exemples.webp",
      },
    ],
  },
  {
    version: "0.7",
    items: [
      {
        title: "Un accueil plus clair",
        text: "Le nouveau logo, le nom et la version de Cahutin en tête, les choix sur toute la largeur, et vos documents récents avec l'aperçu de leur première page : on retrouve le bon d'un coup d'œil.",
        image: "/nouveautes/0.7/accueil.webp",
      },
      {
        title: "Créer votre propre modèle",
        text: "« Partir d'un modèle », puis « Créer un modèle » : choisissez une grille, fusionnez des cases, et enregistrez. Votre modèle rejoint la liste, prêt à resservir.",
        image: "/nouveautes/0.7/modele.webp",
      },
    ],
  },
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
