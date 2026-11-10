// Guides : comment se sert une fonction, en deux temps. Le livret se feuillette (une capture et deux ou trois
// phrases par panneau ; une courte vidéo seulement pour un geste, comme glisser d'une case à l'autre) ; « Me guider » montre ensuite, sur l'écran, le vrai bouton de chaque étape et
// passe à la suivante quand l'action est faite. Pour un nouveau guide : l'ajouter à GUIDES, ses captures dans
// `assets/guides/<id>/`.
import { hasNameFields } from "./classFill";
import { classes } from "./classStore.svelte";
import { isCutMark } from "./cutMarks";
import { editor } from "./docStore.svelte";
import { myImages } from "./myImages.svelte";
import { classLibrary } from "./phototheque.svelte";
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
  {
    id: "classe",
    icon: "🧒",
    title: "Remplir avec ma classe",
    summary: "La liste des prénoms collée une fois, puis une page par enfant ou une planche d'étiquettes, d'un clic.",
    panels: [
      {
        title: "Ouvrir « Ma classe »",
        text: "Accueil > « Ma classe », ou le tiroir Classe > « Modifier la classe… ». La classe reste sur l'ordinateur, dans Documents › Cahutin › Classes.",
        image: img("classe", "ouvrir"),
      },
      {
        title: "Coller la liste des prénoms",
        text: "Un prénom par ligne, copiés d'un tableur ou d'un document, puis « Créer ma classe ». « Ouvrir un fichier… » lit aussi un export d'ONDE : nom, date de naissance, niveau et groupe viennent avec.",
        image: img("classe", "coller"),
      },
      {
        title: "Photos et autorisations",
        text: "Cochez « autorisée » pour les enfants dont les parents ont donné leur accord (« Tout cocher » d'un coup). Sans autorisation, c'est le symbole de l'enfant qui va dans la case photo : « Symboles pour tous » en donne un à chacun.",
        image: img("classe", "autorisations"),
      },
      {
        title: "Une page qui attend des prénoms",
        text: "Un modèle « Pour la classe » (étiquettes, memory, J'écris mon prénom…), ou un champ de la classe posé depuis le tiroir Champs : un bandeau bleu propose alors de remplir la page.",
        image: img("classe", "bandeau"),
      },
      {
        title: "Remplir",
        text: "« Remplir avec… » : Cahutin dit ce qu'il va faire (une planche d'étiquettes, une page par enfant), pour toute la classe, un niveau ou un groupe. Ctrl+Z revient en arrière.",
        image: img("classe", "remplir"),
      },
    ],
    walk: [
      {
        target: '.rail [data-tour="class"]',
        title: "Ouvrez le tiroir Classe",
        text: "Cliquez sur « Classe », dans la colonne de gauche.",
        done: () => editor.drawer === "class" || editor.showClassPage,
      },
      {
        target: '[data-guide="class-edit"]',
        title: "Ouvrez votre classe",
        text: "« Modifier la classe… » ouvre la page « Ma classe ».",
        done: () => editor.showClassPage,
      },
      {
        target: ".class-page textarea, .class-page table",
        title: "Collez la liste",
        text: "Collez les prénoms, un par ligne, puis « Créer ma classe ». Si la classe existe déjà, passez à l'étape suivante.",
        done: () => editor.showClassPage && (classes.current?.class.students.length ?? 0) > 0 && !!document.querySelector(".class-page table"),
      },
      {
        target: '[data-guide="class-consent"]',
        title: "Les autorisations photo",
        text: "Cochez « autorisée » pour chaque enfant dont les parents ont donné leur accord, puis passez à l'étape suivante.",
      },
      {
        target: ".class-page .back",
        title: "Revenez au document",
        text: "« Retour » : la classe est enregistrée.",
        done: () => !editor.showClassPage,
      },
      {
        target: '.rail [data-tour="templates"]',
        title: "Une page à prénoms",
        text: "Dans le tiroir Modèles, « Pour la classe », cliquez sur un modèle (des étiquettes, par exemple).",
        done: () => hasNameFields(editor.page),
      },
      {
        target: '[data-guide="class-fill"]',
        title: "Remplissez",
        text: "« Remplir avec… » dans le bandeau bleu, puis validez.",
      },
    ],
  },
  {
    id: "phototheque",
    icon: "📷",
    title: "Photothèque de la classe",
    summary: "Les photos de l'année rangées par jour, par enfant et par activité, et les enfants peu photographiés.",
    panels: [
      {
        title: "Ouvrir la photothèque",
        text: "Tiroir Classe > « Photothèque », ou le bouton « Photothèque » de la page « Ma classe ». Chaque classe a la sienne, dans Documents › Cahutin › Photos.",
        image: img("phototheque", "ouvrir"),
      },
      {
        title: "Ajouter les photos",
        text: "En haut, « ＋ Photos » ou « 📁 Dossier » : elles se rangent par jour. Le lieu et l'appareil ne sont pas gardés.",
        image: img("phototheque", "ajouter"),
      },
      {
        title: "Qui est sur ces photos ?",
        text: "Choisissez des photos (un clic, Ctrl ou Maj pour plusieurs, ou la date d'un jour), puis cliquez sur les prénoms des enfants présents et sur l'activité. Une nouvelle activité s'ajoute en bas.",
        image: img("phototheque", "qui"),
      },
      {
        title: "Retrouver et équilibrer",
        text: "À gauche, les photos d'un enfant ou d'une activité. En haut, « Peu de photos » signale les enfants peu photographiés sur la période, pour penser à eux.",
        image: img("phototheque", "retrouver"),
      },
      {
        title: "Mettre dans le document",
        text: "« Mettre ces photos dans le document » : elles attendent dans le tiroir Images, onglet Photos. Un enfant sans autorisation est signalé : floutez son visage avant d'imprimer.",
        image: img("phototheque", "document"),
      },
    ],
    walk: [
      {
        target: '.rail [data-tour="class"]',
        title: "Ouvrez le tiroir Classe",
        text: "Cliquez sur « Classe », dans la colonne de gauche.",
        done: () => editor.drawer === "class" || editor.showPhototheque,
      },
      {
        target: '[data-guide="phototheque"]',
        title: "Ouvrez la photothèque",
        text: "Cliquez sur « Photothèque ».",
        done: () => editor.showPhototheque,
      },
      {
        target: '[data-guide="library-add"]',
        title: "Ajoutez des photos",
        text: "« ＋ Photos » ou « 📁 Dossier ». S'il y en a déjà, passez à l'étape suivante.",
        baseline: () => classLibrary.photos.length,
        done: (base) => classLibrary.photos.length > base,
      },
      {
        target: ".library-page .days",
        title: "Choisissez des photos",
        text: "Cliquez sur une photo (Ctrl ou Maj pour plusieurs), ou sur la date d'un jour pour toutes ses photos.",
        done: () => !!document.querySelector(".library-page aside.panel"),
      },
      {
        target: ".library-page aside.panel",
        title: "Qui, et quelle activité ?",
        text: "Cliquez sur les prénoms des enfants présents, puis sur l'activité. « Mettre ces photos dans le document » les envoie au tiroir Images.",
        done: () => !editor.showPhototheque,
      },
    ],
  },
  {
    id: "flou",
    icon: "◍",
    title: "Flouter des visages",
    summary: "Le visage d'un enfant sans autorisation photo, caché avant d'imprimer ; la photo d'origine n'est pas gardée.",
    panels: [
      {
        title: "Choisir la photo",
        text: "Cliquez sur la photo dans la page, puis « Flouter » dans sa barre.",
        image: img("flou", "choisir"),
      },
      {
        title: "Repérage automatique, ou à la main",
        text: "La première fois, Cahutin demande s'il peut repérer les visages tout seul. Le repérage se fait sur l'ordinateur, sans savoir qui est qui, et la photo ne part nulle part. Sinon, vous tracez les zones vous-même.",
        image: img("flou", "accord"),
      },
      {
        title: "Désigner les visages",
        text: "Cliquez sur les visages repérés à flouter. Pour un visage non repéré, tracez une zone en glissant sur la photo ; la croix d'une zone la retire.",
        image: img("flou", "tracer"),
      },
      {
        title: "Flouter",
        text: "« Flouter 1 visage » : le flou est fait, et la photo d'origine ne sera pas enregistrée dans le document. Ctrl+Z revient en arrière tant que le document est ouvert.",
        image: img("flou", "resultat"),
      },
    ],
    walk: [
      {
        target: "main.stage .page",
        title: "Choisissez la photo",
        text: "Cliquez sur la photo à flouter.",
        done: () => editor.selected?.kind.type === "Photo" && !!editor.selected.kind.asset,
      },
      {
        target: '[data-guide="blur"]',
        title: "Flouter",
        text: "Cliquez sur « Flouter » dans la barre de la photo.",
        done: () => !!editor.blurring,
      },
      {
        target: '[data-guide="blur-photo"]',
        title: "Désignez les visages",
        text: "Cliquez sur un visage repéré, ou tracez une zone en glissant sur le visage. Puis « Flouter » en bas.",
        done: () => !editor.blurring,
      },
    ],
  },
  {
    id: "etiquettes",
    icon: "✂️",
    title: "Étiquettes à découper",
    summary: "Une étiquette ou une carte, répétée sur toute la page, avec des repères pour couper droit.",
    panels: [
      {
        title: "Faire une étiquette",
        text: "Un texte avec un cadre (bouton « Cadre »), une image, ou les deux, puis sélectionnez-les ensemble et groupez-les (Ctrl+G) : l'étiquette est un seul bloc. Les modèles Étiquettes en donnent toutes prêtes.",
        image: img("etiquettes", "bloc"),
      },
      {
        title: "Répéter",
        text: "Le bloc choisi, « Répéter » dans sa barre : le nombre de colonnes et de lignes, ou « Remplir la page ». Chaque exemplaire garde ses zones à remplir avec la classe.",
        image: img("etiquettes", "repeter"),
      },
      {
        title: "Repères de découpe",
        text: "Juste après « Répéter », toutes les étiquettes sont choisies : « Repères de découpe » met des traits aux coins, ou des pointillés avec des ciseaux, autour de chacune. Plus tard, tracez un cadre autour d'elles pour les choisir toutes.",
        image: img("etiquettes", "reperes"),
      },
      {
        title: "Imprimer",
        text: "« Imprimer / Exporter ». Pour des cartes recto-verso, mettez les versos sur les pages 2, 4… et « Imprimer en recto-verso » : chaque verso tombe derrière son recto.",
        image: img("etiquettes", "imprimer"),
      },
    ],
    walk: [
      {
        target: "main.stage .page",
        title: "Choisissez l'étiquette",
        text: "Cliquez sur l'étiquette (un bloc groupé, une photo ou un texte).",
        done: () => editor.selection.length > 0,
      },
      {
        target: '[data-guide="repeat"]',
        title: "Répétez-la",
        text: "« Répéter », puis « Remplir la page » ou le nombre d'exemplaires.",
        baseline: () => editor.page.elements.length,
        done: (base) => editor.page.elements.length > base + 1,
      },
      {
        target: '[data-guide="cut"]',
        title: "Ajoutez les repères",
        text: "Les étiquettes sont toutes choisies : « Repères de découpe », puis traits aux coins ou pointillés.",
        done: () => editor.page.elements.some(isCutMark),
      },
      {
        target: ".top",
        title: "Imprimez",
        text: "« Imprimer / Exporter », en haut à droite.",
      },
    ],
  },
  {
    id: "import",
    icon: "📥",
    title: "Reprendre une ancienne fiche",
    summary: "Une fiche Word, PDF, Publisher ou LibreOffice ouverte dans Cahutin, textes et photos modifiables.",
    panels: [
      {
        title: "Importer",
        text: "Fichier > « Importer… », ou Accueil > « Importer un fichier », puis choisissez la fiche. Pour Word, PDF et Publisher, LibreOffice doit être installé sur l'ordinateur : Cahutin s'en sert pour lire le fichier.",
        image: img("import", "importer"),
      },
      {
        title: "Ce qui devient modifiable",
        text: "Les textes, les photos, les tableaux et les tracés (cadres, lignes de graphisme) arrivent à leur place et se modifient comme les vôtres. Une illustration faite de très nombreux tracés devient une image fixe.",
        image: img("import", "resultat"),
      },
      {
        title: "Ce qu'il faut vérifier",
        text: "Un compte rendu dit ce qui a été repris. Relisez la page : une police absente de l'ordinateur est remplacée, et un texte peut déborder de son cadre (il est alors entouré de rouge).",
        image: img("import", "verifier"),
      },
    ],
    walk: [],
  },
  {
    id: "frise",
    icon: "🌸",
    title: "Frise autour d'un bloc",
    summary: "Des images répétées autour d'une photo ou d'un texte, réglables après coup.",
    panels: [
      {
        title: "Choisir les images",
        text: "Tiroir Décors > Frises > « Choisir les images » : des pictogrammes ou les images de votre collection, dans l'ordre où elles se suivront. « Taille » et « Espacement » se règlent dessous.",
        image: img("frise", "motifs"),
      },
      {
        title: "Entourer un bloc",
        text: "Cliquez sur la photo ou le texte, puis « Frise tout autour » dans sa barre : les images l'entourent. Les boutons « En haut », « En bas », « Tout autour » du tiroir font une frise au bord de la page.",
        image: img("frise", "autour"),
      },
      {
        title: "Régler après coup",
        text: "Cliquez sur la frise, puis « Modifier la frise » dans sa barre : la taille des images, l'espace entre elles, ou les images elles-mêmes changent, sans tout refaire. « Supprimer la frise » la retire, en gardant ce qu'elle entoure.",
        image: img("frise", "taille"),
      },
    ],
    walk: [
      {
        target: '.rail [data-tour="decor"]',
        title: "Ouvrez le tiroir Décors",
        text: "Cliquez sur « Décors », dans la colonne de gauche.",
        done: () => editor.drawer === "decor",
      },
      {
        target: '[data-guide="frieze-pick"]',
        title: "Choisissez les images",
        text: "« Choisir les images », cliquez celles de la frise, puis « Terminé ».",
      },
      {
        target: "main.stage .page",
        title: "Choisissez le bloc",
        text: "Cliquez sur la photo ou le texte à entourer.",
        done: () => editor.selection.length > 0,
      },
      {
        target: '[data-guide="frieze-around"]',
        title: "Entourez-le",
        text: "« Frise tout autour » dans sa barre.",
        baseline: () => editor.page.elements.length,
        done: (base) => editor.page.elements.length > base,
      },
      {
        target: '[data-guide="frieze-size"]',
        title: "Réglez",
        text: "« Modifier la frise » change la taille des images, leur espacement, ou les images elles-mêmes.",
      },
    ],
  },
  {
    id: "collection",
    icon: "🗂",
    title: "Ma collection",
    summary: "Vos dessins et illustrations, gardés pour tous vos documents et rangés par thème.",
    panels: [
      {
        title: "Ajouter des images",
        text: "Tiroir Images > onglet Collection > « Ajouter des images ». Une image déjà posée sur la page s'y ajoute avec « Ajouter à ma collection » dans sa barre. Elles sont gardées dans Documents › Cahutin › Images.",
        image: img("collection", "ajouter"),
      },
      {
        title: "Ranger par thème",
        text: "« Organiser ma collection » (ou Accueil > « Ma collection ») : cliquez des images, écrivez un thème (« Automne », « Animaux »), puis « Ranger ». Un clic sur un nom le change.",
        image: img("collection", "ranger"),
      },
      {
        title: "Retrouver et poser",
        text: "Dans le tiroir, les thèmes trient la collection. Un clic pose l'image, ou glissez-la sur la page. Une image retirée n'est pas effacée : « Retirées », puis « Remettre ».",
        image: img("collection", "poser"),
      },
    ],
    walk: [
      {
        target: '.rail [data-tour="images"]',
        title: "Ouvrez le tiroir Images",
        text: "Cliquez sur « Images », dans la colonne de gauche.",
        done: () => editor.drawer === "images",
      },
      {
        target: '[data-guide="tab-mine"]',
        title: "L'onglet Collection",
        text: "Cliquez sur « Collection ».",
        done: () => editor.drawer === "images" && editor.imagesTab === "mine",
      },
      {
        target: '[data-guide="collection-add"]',
        title: "Ajoutez des images",
        text: "« Ajouter des images » et choisissez vos dessins. S'il y en a déjà, passez à l'étape suivante.",
        baseline: () => myImages.images.length,
        done: (base) => myImages.images.length > base,
      },
      {
        target: '[data-guide="collection-organize"]',
        title: "Rangez-les",
        text: "« Organiser ma collection » ouvre la page où l'on range par thème.",
        done: () => editor.showCollection,
      },
      {
        target: ".collection-page",
        title: "Un thème",
        text: "Cliquez des images, écrivez un thème, puis « Ranger ».",
      },
    ],
  },
  {
    id: "assistant",
    icon: "✦",
    title: "L'assistant IA",
    summary: "Facultatif : demander une mise en page en quelques mots, puis l'accepter ou la refuser.",
    panels: [
      {
        title: "L'activer, si vous le voulez",
        text: "Préférences > Assistant de mise en page. Cahutin ne fournit pas l'IA : choisissez votre service (Claude, OpenAI, Gemini, souvent payants, ou Ollama sur l'ordinateur), puis « Activer l'assistant ». Ce cadre dit ce qui part vers ce service : la disposition de la page et vos demandes, jamais vos textes, vos photos ni les prénoms.",
        image: img("assistant", "activer"),
      },
      {
        title: "Demander",
        text: "Le bouton « ✦ Assistant », en haut, ouvre son panneau. Écrivez ce que vous voulez changer, comme à un collègue, puis « Envoyer ». Cliquez d'abord sur un élément pour le désigner. Le panneau vide propose des idées : un clic en met une dans le champ. Si la réponse n'arrive pas, « ⟳ Réessayer » renvoie la même demande. N'écrivez pas de prénom d'élève dans une demande.",
        image: img("assistant", "demander"),
      },
      {
        title: "Accepter ou refuser",
        text: "La proposition s'affiche sur la page, entourée de pointillés, mais rien n'est encore changé. « Accepter » la garde (Ctrl+Z la défait ensuite), « Refuser » ou Ctrl+Z revient à la page d'avant. Pour n'en garder qu'une partie, « Défaire » à côté d'une modification la retire et garde les autres. Relisez les textes écrits par l'IA avant d'imprimer.",
        image: img("assistant", "accepter"),
      },
      {
        title: "Signaler une réponse",
        text: "Une réponse déplacée ou choquante ? « ⚑ Signaler », sous la réponse, prépare dans votre messagerie un e-mail à l'auteur de Cahutin, avec votre demande et la réponse : relisez-le, puis envoyez-le.",
        image: img("assistant", "signaler"),
      },
    ],
    walk: [],
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
    // Lancé depuis l'accueil : il se ferme, les boutons à montrer sont derrière.
    editor.showWelcome = false;
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
