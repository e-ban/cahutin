// Astuces à feuilleter (menu Aide > Astuces), rangées par thème. Chacune décrit une fonction qui existe.
export type TipTheme = "Photos" | "Texte" | "Mise en page" | "Pages et modèles" | "Classe" | "Fichiers";

export interface Tip {
  id: string;
  theme: TipTheme;
  icon: string;
  title: string;
  text: string;
}

export const TIP_THEMES: TipTheme[] = ["Photos", "Texte", "Mise en page", "Pages et modèles", "Classe", "Fichiers"];

export const TIPS: Tip[] = [
  { id: "drop-photo", theme: "Photos", icon: "🖼️", title: "Placer une photo", text: "Importez vos photos dans le tiroir Photos, puis glissez-les sur une zone de la page." },
  { id: "crop", theme: "Photos", icon: "✂️", title: "Recadrer", text: "Double-cliquez sur une photo, puis faites-la glisser dans son cadre ; la molette zoome. Entrée ou un clic ailleurs valide." },
  { id: "photo-style", theme: "Photos", icon: "🟦", title: "Coins, bordure, ombre", text: "Sélectionnez une photo : la barre flottante arrondit ses coins, ajoute une bordure (pleine, en tirets ou en pointillés) ou une ombre." },
  { id: "mirror", theme: "Photos", icon: "⇋", title: "Miroir", text: "Le bouton « Miroir » de la barre de la photo la retourne de gauche à droite : un enfant qui regarde vers le texte, par exemple." },
  { id: "blur-faces", theme: "Photos", icon: "◍", title: "Flouter des visages", text: "Sélectionnez une photo, puis « Flouter » : cliquez sur les visages des enfants sans autorisation photo, ou tracez une zone en glissant. Le repérage automatique (un petit modèle d'intelligence artificielle intégré, hors ligne) n'est utilisé qu'avec votre accord, demandé la première fois, et peut être coupé. La photo d'origine n'est plus enregistrée dans le document." },
  { id: "caption", theme: "Photos", icon: "🏷️", title: "Légende", text: "Le bouton « Légende » ajoute un texte sous la photo. Sur une photo libre, la légende est groupée avec elle et la suit quand on la déplace." },
  { id: "library", theme: "Photos", icon: "📚", title: "Photothèque", text: "Les photos importées mais pas encore placées restent dans le tiroir Photos. À la réouverture du document, elles reviennent si leurs fichiers sont toujours sur l'ordinateur." },
  { id: "write", theme: "Texte", icon: "✏️", title: "Écrire", text: "Double-cliquez dans un texte pour écrire. Échap ou un clic ailleurs termine." },
  { id: "fonts-mix", theme: "Texte", icon: "🔤", title: "Plusieurs polices dans un texte", text: "Pendant l'écriture, sélectionnez un mot et choisissez une police : seul ce mot change. Pratique pour un prénom en cursive et en script." },
  { id: "trace", theme: "Texte", icon: "✍️", title: "Lettres à repasser", text: "Tiroir Texte : « À repasser (gris clair) » ou « Lettres creuses », en cursive. Le bouton « A » creux de la barre flottante rend creuses les lettres de n'importe quel texte." },
  { id: "writing-lines", theme: "Texte", icon: "📝", title: "Lignes d'écriture", text: "Tiroir Texte > « Lignes d'écriture » : un cadre Seyès, à lignes ou à petits carreaux, pour écrire à la main. Le réglage « Cadre » d'un texte en ajoute aussi sous un modèle à repasser." },
  { id: "styles", theme: "Texte", icon: "🅰️", title: "Styles de texte", text: "Le bouton « Style » de la barre d'écriture donne à un texte l'allure Titre, Consigne, Texte ou Prénom. Modifiez un texte, puis « Mettre à jour » : tous les textes de ce style changent avec lui. « Nouveau style… » garde l'allure d'un texte pour la réutiliser." },
  { id: "valign", theme: "Texte", icon: "↕️", title: "En haut, au milieu, en bas", text: "Le menu d'alignement de la barre flottante place aussi le texte en haut, au milieu ou en bas de son cadre." },
  { id: "label", theme: "Texte", icon: "🪧", title: "Étiquettes", text: "Le bouton « Cadre » donne au texte un fond, une bordure, des coins arrondis, ou une image de fond." },
  { id: "system-fonts", theme: "Texte", icon: "💻", title: "Polices de l'ordinateur", text: "Le choix de police propose aussi les polices installées sur l'ordinateur, avec une recherche." },
  { id: "centre", theme: "Mise en page", icon: "📏", title: "Centrer", text: "En déplaçant un bloc, une ligne bleue apparaît quand il est centré sur la page ou aligné sur un autre bloc." },
  { id: "group", theme: "Mise en page", icon: "🧩", title: "Grouper", text: "Sélectionnez plusieurs éléments puis Ctrl+G : ils se déplacent et s'agrandissent ensemble. Ctrl+Maj+G les sépare." },
  { id: "box-select", theme: "Mise en page", icon: "⬚", title: "Sélectionner plusieurs éléments", text: "Tracez un cadre autour des éléments, même en partant de la marge grise, ou cliquez en maintenant Maj." },
  { id: "nudge", theme: "Mise en page", icon: "⌨️", title: "Déplacer au clavier", text: "Les flèches déplacent l'élément choisi de 1 mm, de 5 mm avec Maj." },
  { id: "lock", theme: "Mise en page", icon: "🔒", title: "Verrouiller", text: "Un élément verrouillé (clic droit ou Réglages) ne bouge plus par mégarde." },
  { id: "table", theme: "Mise en page", icon: "▦", title: "Tableaux", text: "Tiroir Texte > « Ajouter un tableau ». Double-cliquez dans une case pour écrire ; la barre sous la case ajoute ou retire des lignes et des colonnes, fusionne des cases et y met un picto ou une photo." },
  { id: "cut-marks", theme: "Mise en page", icon: "✂️", title: "Repères de découpe", text: "Sélectionnez une ou plusieurs étiquettes, puis « Découpe » dans la barre flottante : traits aux coins ou pointillés avec des ciseaux, autour de chaque bloc. « Aucun repère » les retire." },
  { id: "repeat", theme: "Mise en page", icon: "⊞", title: "Répéter un bloc", text: "Sélectionnez une étiquette ou une carte, puis « Répéter » : choisissez les colonnes et les lignes, ou « Remplir la page ». Chaque exemplaire garde ses zones à remplir avec la classe." },
  { id: "frieze", theme: "Mise en page", icon: "🌸", title: "Frise autour d'un bloc", text: "Choisissez des images dans Décors, sélectionnez une photo ou un texte, puis « Frise » : les images l'entourent." },
  { id: "templates", theme: "Pages et modèles", icon: "▦", title: "Partir d'un modèle", text: "Le tiroir Modèles ajoute une page toute prête : grille de photos, récit, étiquettes, affiche." },
  { id: "save-template", theme: "Pages et modèles", icon: "💾", title: "Vos propres modèles", text: "Bouton « Modèle » en haut, puis « Enregistrer comme modèle » : la mise en page devient un modèle à réutiliser ou à partager. Les photos et les textes écrits ne sont pas gardés." },
  { id: "background", theme: "Pages et modèles", icon: "📒", title: "Fond de page", text: "Le bouton « Fond » de la bande des pages donne une couleur, des lignes Seyès, des petits carreaux ou du papier kraft." },
  { id: "landscape", theme: "Pages et modèles", icon: "🔄", title: "Paysage", text: "Le bouton « Paysage » de la bande des pages tourne la page courante." },
  { id: "formats", theme: "Pages et modèles", icon: "📐", title: "A5, A4, A3", text: "Le choix du format, dans la bande des pages, agrandit ou réduit la page avec son contenu. Une affiche A3 s'imprime aussi en deux feuilles A4 à coller (Imprimer / Exporter)." },
  { id: "spread", theme: "Pages et modèles", icon: "📖", title: "Pages en vis-à-vis", text: "« Vis-à-vis » montre les pages deux par deux, comme un cahier ouvert. Pour des cartes recto-verso, « Imprimer en recto-verso » place chaque verso derrière son recto." },
  { id: "class", theme: "Classe", icon: "🧒", title: "Votre classe", text: "« Ma classe » sur l'accueil (ou « Modifier la classe… » dans le tiroir Classe) : collez la liste des prénoms, puis réglez chaque élève sur sa ligne. « Ajouter les photos… » les prend toutes d'un coup, chacune pour l'enfant dont elle porte le prénom (Léa.jpg). La classe reste sur l'ordinateur, dans Documents/Cahutin/Classes." },
  { id: "class-generate", theme: "Classe", icon: "✨", title: "Une page pour chaque élève", text: "Sur une page qui attend des prénoms (modèles Étiquettes, ou le bouton « Prénom » pendant l'écriture), le bandeau bleu propose de la remplir avec votre classe. Une étiquette reçoit le même enfant dans toutes ses écritures." },
  { id: "class-consent", theme: "Classe", icon: "🦊", title: "Sans autorisation photo", text: "Une photo d'élève n'est utilisée que si la case d'autorisation est cochée. Sinon, c'est son symbole qui va dans la zone photo : donnez un symbole à chaque enfant." },
  { id: "class-fields", theme: "Classe", icon: "🎂", title: "Âge, anniversaire, groupe", text: "Pendant l'écriture, le ▾ à côté de « Prénom » insère {nom}, {groupe}, {âge} ou {anniversaire}, remplis pour chaque élève. Les dates de naissance se saisissent sur la page « Ma classe »." },
  { id: "class-templates", theme: "Classe", icon: "🃏", title: "Modèles pour la classe", text: "Partir d'un modèle > « Pour la classe » : memory et loto des prénoms, lettres mobiles, J'écris mon prénom, tableau des présences, affiche des anniversaires. Remplissez-les d'un clic avec votre classe." },
  { id: "class-import", theme: "Classe", icon: "📄", title: "Importer une liste", text: "Sur la page « Ma classe », collez un tableau depuis un tableur, ou « Ouvrir un fichier… » pour un export ONDE (.csv) : prénom, nom, date de naissance, niveau et groupe sont reconnus." },
  { id: "home", theme: "Fichiers", icon: "🏠", title: "Revenir à l'accueil", text: "Un clic sur le logo Cahutin, en haut à gauche (ou Fichier > Page d'accueil), ouvre l'accueil sans fermer le document : « Retour » y ramène. Un modèle ou une page vierge choisis là partent d'un nouveau document." },
  { id: "import", theme: "Fichiers", icon: "📥", title: "Reprendre une ancienne fiche", text: "Fichier > Importer reprend une fiche Word, PDF ou Publisher (LibreOffice doit être installé) ou les pages d'un autre carnet." },
  { id: "odg", theme: "Fichiers", icon: "📤", title: "Exporter pour LibreOffice", text: "« Imprimer / Exporter » crée un fichier LibreOffice Draw où chaque texte et chaque photo restent modifiables. Les polices voyagent avec le fichier, sauf celles marquées « ⚠ export »." },
  { id: "undo", theme: "Fichiers", icon: "↶", title: "Revenir en arrière", text: "Ctrl+Z annule la dernière action, Ctrl+Y la rétablit." },
];
