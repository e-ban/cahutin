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
    version: "0.16",
    items: [
      {
        title: "Un nouveau visage",
        text: "Des icônes livrées avec Cahutin, les mêmes sur tous les ordinateurs. L'accueil se range en deux colonnes : « Commencer » un document, ou « Reprendre » un document récent. En bas de la fenêtre, toutes les commandes de la page restent visibles sur une seule ligne.",
        image: "/nouveautes/0.16/look.webp",
      },
      {
        title: "Des thèmes, dont un thème sombre",
        text: "Préférences > Affichage : Papier (les couleurs d'origine), Craie, Crayon, ou Nuit pour travailler le soir. La page, elle, reste toujours blanche, comme sur papier.",
        image: "/nouveautes/0.16/themes.webp",
      },
      {
        title: "Des préférences mieux rangées",
        text: "Les réglages se rangent par catégories, à gauche de la fenêtre. « Chercher un réglage » trouve le bon en tapant un mot : « contraste », « sombre », « A4 »… Les Préférences s'ouvrent aussi depuis l'accueil.",
        image: "/nouveautes/0.16/preferences.webp",
      },
      {
        title: "Choisir ses élèves",
        text: "Sur la page Ma classe, cochez des élèves pour leur donner le même niveau ou le même groupe d'un coup. « Remplir avec ma classe » permet aussi de choisir les élèves un par un.",
        image: "/nouveautes/0.16/eleves.webp",
      },
    ],
  },
  {
    version: "0.13",
    items: [
      {
        title: "Un assistant IA, si vous le voulez",
        text: "Facultatif et désactivé au départ (Préférences > Assistant de mise en page). Écrivez ce que vous voulez, « mets le titre en rouge », « aligne les images », et l'assistant propose : rien ne change avant « Accepter ». Cahutin ne fournit pas l'IA, il se branche sur votre propre service, et ne lui envoie ni vos textes, ni vos photos, ni les prénoms. Le guide « L'assistant IA » montre tout.",
        image: "/nouveautes/0.13/assistant.webp",
      },
      {
        title: "Des champs qui se remplissent seuls",
        text: "Le tiroir Champs pose le prénom de chaque enfant, son âge, la date, la saison, le nom de la classe ou de l'école. Ils se remplissent à la création de la page, ou avec la classe ; un champ pas encore rempli s'imprime en blanc.",
        image: "/nouveautes/0.13/champs.webp",
      },
      {
        title: "Une copie de secours",
        text: "Toutes les deux minutes, un document pas encore enregistré est copié. Si Cahutin s'arrête (panne, coupure de courant), il propose au lancement suivant de reprendre ce travail.",
        image: "/nouveautes/0.13/secours.webp",
      },
      {
        title: "Une barre plus courte",
        text: "Les outils moins utilisés (miroir, quarts de tour, répéter, repères de découpe) sont rangés dans « ⋯ Plus ». Pour tout voir d'un coup d'œil, décochez ce réglage dans les Préférences.",
        image: "/nouveautes/0.13/plus.webp",
      },
    ],
  },
  {
    version: "0.12",
    items: [
      {
        title: "Rogner une photo",
        text: "Double-cliquez sur une photo : en plus de la glisser et de l'agrandir à la molette, tirez les poignées orange pour couper ses bords. La partie coupée reste visible en transparence. Agrandie ensuite par un coin, la photo garde son recadrage (Maj pour la déformer).",
        image: "/nouveautes/0.12/rogner.webp",
      },
      {
        title: "Des réglages bien rangés",
        text: "Le panneau Réglages se range en sections qui se replient d'un clic. Pour une photo, « Recadrage » règle au millimètre ce que coupe chaque bord, l'agrandissement et la position ; « Photo entière » la remontre en entier.",
        image: "/nouveautes/0.12/reglages.webp",
      },
      {
        title: "Modifier une frise",
        text: "Cliquez sur une frise : « Modifier la frise » change la taille des images, leur espacement, la marge avec le bord, ou les images elles-mêmes. « Supprimer la frise » la retire, en gardant ce qu'elle entoure.",
        image: "/nouveautes/0.12/frise.webp",
      },
      {
        title: "Enregistrer en un clic",
        text: "Le bouton « Enregistrer » est à côté du titre, et le titre devient le nom du fichier. Sous Windows, un double-clic sur un document .carnet ouvre Cahutin.",
        image: "/nouveautes/0.12/enregistrer.webp",
      },
      {
        title: "Plus confortable",
        text: "Des Préférences plus larges, sur deux colonnes. La molette fait défiler les pages en bas, Ctrl et la molette zooment, et un texte copié garde sa police et sa taille quand on le colle dans une autre zone.",
        image: "/nouveautes/0.12/preferences.webp",
      },
    ],
  },
  {
    version: "0.11",
    items: [
      {
        title: "Des guides, pas à pas",
        text: "Accueil > « Guides », ou « ? Comment faire » là où vous travaillez : créer un modèle, remplir avec sa classe, flouter un visage, faire des étiquettes à découper… À la fin, « Me guider » montre chaque bouton sur l'écran.",
        image: "/nouveautes/0.11/guides.webp",
      },
      {
        title: "Cahutin à votre vue",
        text: "Le bouton « Préférences », en haut : une interface plus grande, une police très lisible ou pour les lecteurs dyslexiques, un contraste élevé, de grandes poignées pour un écran tactile, et le format d'une nouvelle page.",
        image: "/nouveautes/0.11/preferences.webp",
      },
      {
        title: "Pensé pour les daltoniens",
        text: "Chaque couleur dit son nom, ce qui ne se voyait qu'à la couleur a aussi un signe, et des couleurs que tous les élèves distinguent. Le bouton « Vision daltonienne » montre la page comme la voit un enfant daltonien.",
        image: "/nouveautes/0.11/daltonisme.webp",
      },
      {
        title: "Tous les outils sous la main",
        text: "La barre au-dessus de l'élément montre tous ses outils, avec leur nom. Un seul outil « Miroir » retourne photos, images, formes et groupes ; la rotation se fait par crans de 15° ou par quarts de tour.",
        image: "/nouveautes/0.11/barre.webp",
      },
      {
        title: "Un modèle plus simple à faire",
        text: "En mode modèle, chaque texte et chaque image est « À remplir » (vidé à chaque utilisation) ou « Fixe » (un titre, le logo de l'école). Ce qui n'est ni l'un ni l'autre n'est pas gardé.",
        image: "/nouveautes/0.11/modele.webp",
      },
    ],
  },
  {
    version: "0.10",
    items: [
      {
        title: "Graphisme, labyrinthes, symétrie",
        text: "Tiroir Formes : des lignes de graphisme à repasser (ponts, vagues, boucles…) à la taille de la section, un labyrinthe différent à chaque clic, des dessins à compléter par symétrie et des repères pour écrire.",
        image: "/nouveautes/0.10/graphisme.webp",
      },
      {
        title: "Des images à colorier",
        text: "Tiroir Images > Pictogrammes : cochez « À colorier » pour poser les images en contour noir seul, à colorier ou à repasser.",
        image: "/nouveautes/0.10/colorier.webp",
      },
      {
        title: "Toutes les images au même endroit",
        text: "Le tiroir Photos devient Images : vos photos, votre collection et les pictogrammes. Vos propres dessins y restent pour tous vos documents ; une image posée s'y ajoute avec « Ajouter à ma collection ».",
        image: "/nouveautes/0.10/images.webp",
      },
      {
        title: "Votre collection, rangée par thème",
        text: "Accueil > « Ma collection » : choisissez des images et rangez-les dans un thème (« Automne », « Animaux »), renommez-les, remettez celles retirées. Les thèmes se retrouvent dans le tiroir Images.",
        image: "/nouveautes/0.10/collection.webp",
      },
      {
        title: "Des frises à votre goût",
        text: "Une frise peut mêler pictogrammes et images de votre collection. Réglez la taille et l'espacement dans Décors, ou après coup : sélectionnez la frise, puis « Frise » dans sa barre.",
        image: "/nouveautes/0.10/frise.webp",
      },
    ],
  },
  {
    version: "0.9",
    items: [
      {
        title: "La photothèque de la classe",
        text: "Toutes les photos de l'année au même endroit, rangées par jour. Indiquez qui est sur chaque photo et l'activité, retrouvez les photos d'un enfant ou d'une sortie, et un bandeau vous signale les enfants peu photographiés. « Ma classe » > Photothèque.",
        image: "/nouveautes/0.9/phototheque.webp",
      },
      {
        title: "Trier vite, même à 40",
        text: "Double-cliquez sur une photo pour la voir en grand et cocher les prénoms, les flèches passent à la suivante. Ou « Marquer un enfant » : choisissez un prénom, puis cliquez sur ses photos.",
        image: "/nouveautes/0.9/trier.webp",
      },
      {
        title: "Une grande classe en quelques clics",
        text: "« Tout cocher » pour l'autorisation photo, « Symboles pour tous » pour donner un symbole différent à chaque enfant. Dans un document de 40 pages, tapez un numéro ou un prénom pour aller à la bonne page.",
        image: "/nouveautes/0.9/classe.webp",
      },
      {
        title: "Vos anciennes fiches, reprises fidèlement",
        text: "Une fiche PDF garde ses tracés (graphisme, cadres), une police absente est remplacée sans déborder, les listes gardent leurs puces, et un fichier LibreOffice Draw revient tel quel.",
        image: "/nouveautes/0.9/import.webp",
      },
      {
        title: "Toutes les couleurs de peau",
        text: "Tiroir Décors : les pastilles « Peau » donnent aux personnes et aux mains la couleur de peau choisie.",
        image: "/nouveautes/0.9/peau.webp",
      },
    ],
  },
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
