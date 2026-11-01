# Journal des versions

## Non publié

- Groupes dans des groupes : grouper des éléments déjà groupés (une frise et une photo avec sa légende) les garde entiers dans le nouveau groupe, et « Dissocier » n'enlève plus qu'un niveau : on retrouve la frise et la photo avec sa légende, chacune toujours groupée. Un double-clic entre d'un niveau à la fois (puis modifie l'élément seul), Échap remonte d'un niveau. Une frise posée autour d'un bloc forme un groupe à elle dans ce groupe. Ces niveaux sont gardés à l'export LibreOffice ; un document qui en contient demande cette version de Cahutin.
- Tiroir Photos : une croix en haut à droite de chaque photo la retire du document, après confirmation. Si elle est posée sur une page, sa case reste, vide ; le fichier sur l'ordinateur n'est pas touché, et « Annuler » la remet.
- Mes images : dans le tiroir Décors, « Mes images » garde vos propres images (dessins téléchargés, cliparts, images faites à la main) pour tous vos documents. « Ajouter mes images » les choisit sur l'ordinateur ; un clic en pose une, ou on la glisse sur la page. Une image retirée de la liste n'est pas effacée : elle est rangée dans le sous-dossier Retirées. Elles sont rangées dans Documents/Cahutin/Images ; une image posée est copiée dans le document, qui reste complet si on l'envoie à une collègue ou si l'image est retirée de « Mes images ».
- Une photo posée deux fois de suite d'un clic se décale un peu au lieu de se cacher sous la première. La barre d'une photo n'affiche plus deux boutons « Remplacer ».
- Fiches de graphisme : tiroir Formes > Graphisme. Des lignes de ponts, coupes, vagues, boucles, créneaux, dents de scie, traits ou ronds à repasser, à la taille de la section (PS, MS, GS) : un point vert pour commencer, le modèle en trait plein, la suite en pointillés gris. Et des « Chemins à suivre » en pointillés, à étirer entre deux images. Ces tracés s'impriment et s'exportent pour LibreOffice tels quels ; un document qui en contient demande cette version de Cahutin.
- Labyrinthes et symétrie : tiroir Formes. « Nouveau labyrinthe » en crée un différent à chaque clic (4, 6 ou 9 cases de côté selon la section), avec un poussin à l'entrée et sa maman à la sortie. « Symétrie » pose un quadrillage, l'axe en rouge et la moitié d'un dessin (sapin, maison, fusée, bateau, cœur) à compléter. « Repères pour écrire » : un point vert de départ et une flèche verte de sens, à poser sur les lettres à repasser.
- Import d'un PDF : les tracés (graphisme, cadres arrondis) deviennent des formes modifiables, qu'on peut recolorer, déplacer ou supprimer une à une ; au-delà de 60 tracés sur une page (une illustration), ils restent une image fixe, plus fidèle.
- Images à colorier : dans le tiroir Décors, « À colorier » pose les images en contour noir seul (toutes les images d'OpenMoji), pour les coloriages et les contours à repasser. Une image déjà posée bascule avec « À colorier » dans sa barre flottante. Un document qui en contient demande cette version de Cahutin.
- Les dessins SVG (le format le plus courant des dessins libres à télécharger) s'importent comme des photos, avec leur transparence.
- Tiroir Photos : un clic pose une photo de plus, à ses proportions ; il ne remplace plus la photo choisie (ce qui recadrait la nouvelle dans l'ancien cadre). Une case photo de modèle se remplit toujours d'un clic, et « Remplacer » dans la barre d'une photo la change.
- Un clic dans le tiroir Décors pose toujours une nouvelle image : il ne remplace plus l'image choisie par mégarde. Pour changer une image, « Remplacer » dans sa barre flottante.
- Un texte ajouté depuis le tiroir Texte (« Titre »…) s'ouvre avec son texte modèle sélectionné : on tape directement le sien.
- Deux éléments ajoutés de suite ne se posent plus exactement l'un sur l'autre.

## 0.9.0

- Photothèque de la classe (page « Ma classe », tiroir Photos ou tiroir Classe) : toutes les photos de l'année, rangées dans Documents/Cahutin/Photos, sans lieu ni appareil, avec leur date de prise de vue. On trie un lot à l'arrivée (qui est sur ces photos ? quelle activité ?), on retrouve les photos d'un enfant ou d'une sortie, sur le mois, trois mois ou l'année, et on les met dans le document. Un bandeau signale les enfants peu photographiés ; une pastille, les photos d'un enfant sans autorisation, et Cahutin rappelle de flouter son visage quand on en pose une. Aucun visage n'est reconnu : on indique les enfants d'un clic.
- Pour une classe de 40 (essais avec 40 enfants et 300 photos) :
  - Photothèque : un double-clic montre la photo en grand, avec les prénoms à cocher et les flèches pour passer à la suivante. « Marquer un enfant » : on choisit un prénom, puis on clique sur ses photos. Les photos sont rangées par jour (un clic sur la date choisit le jour), Ctrl+A les choisit toutes, les prénoms sont dans l'ordre alphabétique, et deux « Anna » se distinguent par l'initiale de leur nom (« Anna D. », « Anna M. »). Le bandeau des enfants peu photographiés en montre six, puis « et 12 autres ».
  - Un lot de photos apparaît photo après photo pendant l'import, et chacune est enregistrée aussitôt : fermer Cahutin en cours de route garde celles déjà ajoutées.
  - Page « Ma classe » : « Tout cocher » pour l'autorisation photo, et « Symboles pour tous » donne un symbole différent à chaque enfant qui n'en a pas.
  - Document de 40 pages : le numéro de la page, ou le prénom d'un enfant, à saisir pour y aller (« Anna » passe d'une Anna à l'autre), Pg.Suiv. et Pg.Préc., une bande de miniatures plus large qui suit la page courante, et le prénom de l'enfant au survol de sa miniature (avec l'initiale du nom pour deux homonymes, quand le nom est sur la page).
  - L'export pour LibreOffice affiche « Export en cours… » pendant qu'il travaille.
  - Les raccourcis de la page (Suppr, flèches, Ctrl+Z) n'agissent plus sur le document caché derrière « Ma classe » ou la photothèque.
- Tiroir Décors : les personnes et les mains existent en cinq couleurs de peau (pastilles « Peau » au-dessus des images), en plus du jaune d'OpenMoji ; le choix est gardé.
- Import plus fidèle (essai avec des documents réels de maternelle) :
  - Les tracés d'un PDF (fiche de graphisme, cadre arrondi) ne sont plus perdus : ils sont redessinés dans une image fixe, au fond de la page, avec leurs couleurs et leurs pointillés.
  - Une police absente de l'ordinateur prend une police de même largeur quand elle existe (Helvetica et Arial : Liberation Sans ; Times : Liberation Serif ; Calibri : Carlito…). Sinon le texte rétrécit juste assez pour tenir, au lieu de déborder, et le message d'import le dit.
  - Les puces des listes Word restent au début de leur ligne.
  - Un fichier LibreOffice Draw (.odg), dont ceux qu'exporte Cahutin, se relit tel quel, sans conversion : pictos, cursive, lettres creuses, alignements, couleurs des cadres et fond de page reviennent, en une fraction de seconde.
- Version AppImage : les images du tiroir Décors s'affichent de nouveau (les cases étaient écrasées et vides avec le moteur web embarqué, cliquables quand même).
- Mode modèle : « Zone photo » et « Zone texte » dans la barre de gauche, pour poser des zones où l'on veut, sans grille (une page partie de « Créer un modèle » n'en avait aucun moyen). « Logo » devient « Image fixe », avec une explication : elle reste sur chaque page faite avec le modèle. En mode modèle, une image ou une case fixée se déplace et se supprime encore.
- Cahutin est sur le [Microsoft Store](https://apps.microsoft.com/detail/9P1VD877KD19) : installation en un clic et mises à jour par le Store (cette version ne consulte jamais GitHub). Le site et le README le proposent en premier pour Windows.
- Site : une page « Confidentialité ».

## 0.8.0

- Accueil depuis l'éditeur : un clic sur le logo Cahutin (ou Fichier > Page d'accueil) l'ouvre par-dessus le document, avec un bouton pour y revenir. Un modèle, une page vierge ou un nouveau modèle choisis depuis là partent d'un nouveau document, après avoir proposé d'enregistrer.
- Six nouveaux exemples : Notre semaine (emploi du temps en tableau), Le loto des émotions, J'écris les jours, Les règles de la classe (affiche A3), Mes cinq sens, Étiquettes de porte-manteau. Les exemples « J'écris mon prénom », « Joyeux anniversaire » et « Les saisons » profitent des lignes d'écriture, des lettres creuses et du champ {âge}.
- Les miniatures des pages en paysage ou en A3 tiennent entières dans le choix des modèles et des exemples.
- Six modèles « Pour la classe », à remplir d'un clic avec la classe : Memory des prénoms (paires photo et prénom), Loto des prénoms, Lettres mobiles du prénom (une case par lettre, en capitales, en script et en cursive), J'écris mon prénom (modèle à repasser et lignes Seyès), Tableau des présences, Affiche des anniversaires (une seule affiche pour toute la classe, avec le nouveau champ {anniversaires mars}).
- Une invite trop longue pour sa zone n'affiche plus « Texte trop long ».
- Pages en vis-à-vis (bande des pages, « Vis-à-vis ») : les pages deux par deux, comme un cahier ouvert ; un clic sur la page d'en face l'ouvre.
- « Imprimer en recto-verso (cartes) » : les pages 2, 4, 6… sont les versos, inversées de gauche à droite pour que chaque carte tombe derrière la sienne.
- Styles de texte (bouton « Style » de la barre d'écriture) : Titre, Consigne, Texte, Prénom, et les vôtres. Un texte modifié peut mettre son style à jour : tous les textes de ce style, dans tout le document, changent avec lui. Un document qui utilise des styles est refusé par Cahutin 0.7 et avant.
- Répéter un bloc en grille (barre de la sélection, « Répéter ») : colonnes × lignes, ou « Remplir la page ». Chaque exemplaire forme un groupe et garde ses zones à remplir : une planche d'étiquettes ou de cartes en deux clics, prête pour la classe.
- Formats A5, A4 et A3 (bande des pages) : le contenu suit la taille de la page. Chaque page s'imprime sur une feuille de son format, et le PDF prend le format de la première page. « Imprimer les affiches A3 sur deux feuilles A4 » sort chaque page A3 en deux moitiés qui se recouvrent de 1 cm, à coller.
- Lignes d'écriture dans un cadre (tiroir Texte : Seyès, lignes, petits carreaux ; ou réglage « Cadre » d'un texte) : pour écrire le prénom ou une phrase sous un modèle. Elles s'impriment et s'exportent ; un document qui en contient est refusé par Cahutin 0.7 et avant.
- Photo en miroir (barre de la photo, « Miroir »), à l'écran, à l'impression et à l'export ; un document qui en contient est refusé par Cahutin 0.7 et avant.
- Décors : quatre catégories pour la classe, en tête de liste : Rituels, Matériel, Émotions, Corps et soins.
- Classe : date de naissance de chaque élève, et deux champs de plus pour les pages générées, {âge} (« 4 ans ») et {anniversaire} (« 12 mars »), à côté de {prénom}, {nom} et {groupe} (bouton ▾ à côté de « Prénom » pendant l'écriture).
- Classe : la liste s'importe d'un tableur ou d'un export ONDE, collée ou ouverte en fichier .csv ; les colonnes prénom, nom, date de naissance, niveau et groupe sont reconnues à leur titre.

## 0.7.0

- Nouveau logo : un lutin qui écrit dans son cahier, à l'abri d'un livre ouvert en tente.
- Accueil : le nom et la version de Cahutin en tête, les choix sur toute la largeur, et les documents récents avec l'aperçu de leur première page.
- « Partir d'un modèle » propose aussi « Créer un modèle » : une page vierge en mode modèle, à enregistrer pour la réutiliser.

## 0.6.0

- Page « Ma classe » (bouton de l'accueil, ou « Modifier la classe… » dans le tiroir Classe) : il suffit de coller les prénoms ; chaque élève tient sur une ligne, avec sa photo, la case d'autorisation des parents, son symbole, et si besoin nom, niveau et groupe. Le tiroir Classe, à côté du document, garde le choix de la classe et le remplissage de la page. Elle reste sur l'ordinateur, dans Documents/Cahutin/Classes, et s'enregistre seule.
- Remplir avec la classe : sur une page qui attend des prénoms, un bandeau propose de la remplir. Une planche d'étiquettes reçoit un enfant par étiquette, dans toutes ses écritures (capitales, script, cursive) ; une autre page est déclinée en une page par enfant. Les pages remplies remplacent la page modèle (Ctrl+Z pour revenir). Les zones photo reçoivent la photo de l'enfant seulement avec l'autorisation, sinon son symbole. Bouton « Prénom » pendant l'écriture pour insérer {prénom}.
- Nouveau modèle « Étiquettes 3 écritures ».
- Photos de la classe ajoutées d'un coup, chacune attribuée à l'enfant dont elle porte le prénom, avec une seule question pour l'autorisation.
- Flouter des visages (barre d'une photo, « Flouter ») : Cahutin repère les visages sur l'ordinateur, sans savoir qui est qui, et on clique sur ceux à flouter (enfants sans autorisation photo) ; une zone se trace aussi à la main. À la première utilisation, Cahutin demande l'accord pour ce repérage par intelligence artificielle, en précisant qu'il se fait hors ligne, par un modèle intégré à l'application et exécuté sur l'ordinateur ; il se coupe ensuite d'une case. Le flou est appliqué à la photo elle-même : l'originale n'est plus enregistrée dans le document ni dans les exports. Repérage par YuNet (OpenCV Zoo, licence MIT).
- Tableaux (tiroir Texte) : une case contient un texte, un pictogramme ou une photo. Double-cliquer dans une case ouvre une barre pour ajouter ou retirer des lignes et des colonnes, fusionner ou diviser des cases. Le tableau sélectionné s'étire librement en largeur ou en hauteur, par les coins et par les bords, sans changer la taille du texte ; les traits se règlent (épaisseur, couleur, tirets, pointillés, aucun). « Dissocier » rend des cases libres et des traits. Un document qui contient un tableau est refusé par Cahutin 0.5 (« version plus récente »).
- Repères de découpe (barre flottante, « Découpe ») : traits aux coins ou pointillés avec des ciseaux, autour de chaque bloc ou groupe sélectionné, pour découper étiquettes et cartes. Ils suivent le bloc, s'impriment et s'exportent.
- Lettres à repasser : styles prêts « À repasser (gris clair) » et « Lettres creuses » dans le tiroir Texte, et bouton « A » creux dans la barre flottante pour n'importe quel texte. Les lettres creuses s'exportent avec l'effet « contour » de LibreOffice ; un document qui en contient est refusé par Cahutin 0.5.
- Après la mise à jour, une fenêtre présente les nouveautés avec des captures d'écran (une fois ; Aide > « Nouveautés » pour la revoir, et retrouver celles des versions précédentes à partir des prochaines).
- Aide : la visite guidée montre la classe ; la page « Ma classe » s'explique à sa première ouverture (« Comment ça marche ? » pour y revenir) ; nouvelles bulles d'aide sur les tableaux et le flou des visages.

## 0.5.2

- Export pour LibreOffice : il échouait sur un document contenant des photos importées.
- Aligner et Répartir déplacent chaque groupe d'un bloc : une photo garde sa légende, une bordure reste entière. Un groupe sélectionné seul n'a rien à aligner.
- Import : le fond blanc des pages n'est plus une forme qu'on sélectionne par erreur, et une bordure décorative découpée en centaines de morceaux devient un seul groupe, à déplacer ou supprimer d'un clic.

## 0.5.1

- Clic droit sur la page : « Ajouter un texte ici » et « Ajouter une forme ici » ne faisaient rien.
- Accueil : « Partir d'un modèle » remplace « Une page de cahier de vie » : les modèles servent aussi aux fiches, affiches et étiquettes.

## 0.5.0

- Carnet s'appelle désormais Cahutin, logiciel libre sous EUPL 1.2. Première version publique, en bêta : retours bienvenus par « Signaler un problème » ou par une issue GitHub.
- Au premier lancement, Cahutin reprend les modèles personnels et les polices ajoutées dans Carnet ; les documents `.carnet` s'ouvrent comme avant. Carnet peut ensuite être désinstallé.
- Avis de nouvelle version : au premier lancement, Cahutin demande s'il doit prévenir quand une nouvelle version sort. Si oui, il lit une fois par jour le numéro de la dernière version sur github.com, sans rien envoyer d'autre, et affiche un bandeau avec un lien vers la page des téléchargements. Réglable dans le menu Aide ; sans accord, Cahutin ne se connecte jamais à internet.
- Sécurité : un fichier `.carnet` piégé ne peut plus écrire en dehors du dossier de l'application ni saturer la mémoire ; il est refusé comme fichier abîmé. Bibliothèques de lecture des fichiers zip et XML mises à jour (failles publiées).
- Windows affiche un avertissement (SmartScreen) à l'installation : cliquer sur « Informations complémentaires », puis « Exécuter quand même ».
- Visite guidée de l'écran au premier lancement, et menu Aide : visite, astuces à feuilleter, réglage pour couper les astuces pendant le travail. « Signaler un problème » et « À propos » passent du menu Fichier au menu Aide.
- Fichier > Importer… (et « Importer un fichier » à l'accueil) : pages d'un autre carnet, et fiches Word, LibreOffice, PDF ou Publisher reprises en textes, photos et formes modifiables, avec un compte rendu. LibreOffice doit être installé pour ces formats.
- Plusieurs polices dans un même texte : pendant l'écriture, la police et la taille choisies dans la barre flottante s'appliquent au passage sélectionné (avant, elles s'appliquaient à tout le texte et terminaient l'écriture). La barre affiche la police du passage sous le curseur.
- Cinq nouveaux modèles : Étiquettes (12 prénoms encadrés à découper), Étiquettes photo (photo et prénom), Photos et textes, Photos légendées, Affiche paysage.
- Frises : « Choisir les images » compose une suite de motifs sans rien poser sur la page ; la frise répète la suite, tout autour en une boucle continue.
- Frise autour d'une photo, d'une forme ou d'un texte, groupée avec lui.
- Texte : alignement vertical (haut, milieu, bas), marge intérieure réglable, cadre (fond, bordure, coins arrondis) et image de fond, pour des étiquettes par exemple.
- Bordures pleines, en tirets ou en pointillés pour les photos, les formes et les cadres de texte.
- Icônes d'alignement à la manière des traitements de texte.
- Un bloc déplacé s'accroche par son centre au milieu de la page et au centre des autres blocs, avec la ligne bleue de guidage ; un bloc accroché se pose exactement sur la ligne (il restait à 0,1 mm).
- Les groupes et les sélections de plusieurs blocs s'accrochent aussi aux lignes, aux autres blocs et à la grille.
- Les menus de la barre flottante restent ouverts pendant un réglage.
- Un document qui utilise ces nouveautés est refusé par Carnet 0.4 (« version plus récente ») au lieu de les perdre.

## 0.4.1

- Version Linux : AppImage pour toutes les distributions et paquet .deb pour Debian et Ubuntu.

## 0.4.0

Première version publiée.

- Pages de cahier de vie en A4 portrait ou paysage, modèles fournis (Grille de photos, Récit, Page à compléter) et modèles personnels.
- Cinq exemples à ouvrir depuis l'écran d'accueil ou Fichier > Exemples.
- Photos : import d'un dossier, photothèque du document, recadrage, coins arrondis, bordure, ombre et légende groupée avec la photo.
- Textes enrichis ; polices fournies, polices ajoutées et polices de l'ordinateur, avec une recherche ; une police intégrable voyage dans le document.
- Formes : rectangle, ellipse, trait, flèche, étoile, cœur, bulle de parole, nuage.
- Images OpenMoji avec recherche en français, cadres de page et frises.
- Fonds de page : couleur, Seyès, petits carreaux, lignes, pointillés, papier kraft.
- Groupes, sélection au cadre (aussi depuis la vue autour de la page), alignement, verrouillage.
- Impression, export PDF et export ODG pour LibreOffice Draw.
