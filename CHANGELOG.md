# Journal des versions

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
