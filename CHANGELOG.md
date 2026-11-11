# Journal des versions


## Non publié

- Fiches de graphisme : dans le tiroir Formes > Graphisme, « Pointillés » (toute la ligne, la moitié, quelques-uns ou aucun, pour que l'enfant continue seul ; ou « De moins en moins, ligne après ligne »), « Lignes guides » (ligne de base, couloir de deux lignes ou aucune) et « Flèche de départ », qui montre dans quel sens commencer.
- Lignes de graphisme déjà posées : on les sélectionne, une ou plusieurs à la fois, pour changer le motif, les pointillés, les lignes guides ou la flèche. On élargit ou on rétrécit une ligne par ses côtés, sans que ses motifs changent de taille. Les motifs vides apparaissent en gris clair à l'écran tant que la ligne est sélectionnée, jamais à l'impression.
- Graphisme : les ronds se tracent dans le sens de l'écriture.
- Graphisme : le point vert est posé juste là où le crayon commence le tracé, et la flèche, à côté, suit le sens du premier geste.
- Assistant : il pose les mêmes lignes de graphisme (pointillés, lignes guides, flèche de départ).

## 0.16.0

- Nouveautés de la 0.16, montrées une fois après la mise à jour : le nouveau visage, les thèmes, les Préférences rangées, choisir ses élèves.
- Ma classe : une case en début de ligne choisit un élève (celle de l'en-tête, toute la classe) ; un bandeau donne alors le même niveau ou le même groupe à tous les élèves choisis.
- Mode modèle : une case photo vide n'affiche plus « Choisir une photo », qui ne faisait rien (un modèle garde ses cases vides), ni le lien vers les photos pour l'image d'un cadre de texte.
- Préférences : les catégories à gauche (Affichage, Accessibilité, Daltonisme, Barre d'outils, Nouvelle page, Graphisme, Exporter, Aide, Assistant), les réglages de celle choisie à droite, et « Chercher un réglage » pour trouver un réglage par son nom (« contraste », « sombre », « A4 »…).
- Frise : le menu « Modifier la frise » reste ouvert pendant qu'on règle la taille, l'espacement ou la marge, et « Supprimer la frise » s'y trouve, à côté de « Changer les images ».
- Thèmes : Préférences > Affichage propose Papier (les couleurs d'origine), Craie, Crayon et Nuit, un thème sombre pour le soir. La page reste blanche, comme sur papier, et le contraste élevé passe avant le thème. Les Préférences s'ouvrent aussi depuis la page d'accueil.
- Des icônes identiques sur Windows, macOS et Linux : l'interface ne passe plus par les emojis du système, mais par un même jeu d'icônes au trait (Lucide), livré avec Cahutin.
- Barre du haut : « Imprimer / Exporter » est le seul bouton bleu ; « Enregistrer » est plus discret, avec le point « pas encore enregistré » dedans ; Annuler et Rétablir vont par paire.
- Colonne de gauche : le tiroir ouvert est surligné.
- Menus : une colonne d'icônes, les raccourcis à droite, des titres de section (« Documents récents ») et des coches pour ce qui est activé.
- Barre de l'élément : les outils sont rangés par familles, dans le même ordre pour tous les éléments (le contenu, l'apparence, la place sur la page, puis Supprimer).
- Panneau Réglages : des titres plus calmes, plus d'air, et la valeur écrite à côté de chaque curseur.
- Barre du bas : une seule ligne, plus basse ; les vignettes portent leur numéro ; les commandes de la page restent visibles, en deux groupes (Ajouter, Dupliquer, Supprimer ; Fond, format, Paysage, Vis-à-vis).
- Accueil : deux colonnes, « Commencer » (partir d'un modèle, page vierge, exemple, importer) et « Reprendre » (les documents récents en grand, ou en ouvrir un autre), puis « Mes ressources » (Ma classe, Ma collection, Guides), avec des pictogrammes en couleur.
- Remplir avec ma classe : « Choisir les élèves » déplie la liste de la classe, à cocher un par un, avec « Tous » et « Aucun ». Le niveau ou le groupe cochent leurs élèves d'un coup, et le nombre de pages suit.
- Barre de l'élément : un menu qui s'ouvre sous un outil (Champ, Styles, Plus, polices…) ne dépasse plus de la fenêtre. Il s'ouvre au-dessus quand la place manque en dessous, et défile s'il est trop long.
- Import d'un PDF : un mot ou une phrase soulignés arrivent soulignés, au lieu d'un trait à part sous le texte. Le compte rendu ne compte plus les cadres vides, et quand beaucoup de dessins manquent, il le dit sans chiffre (« comparez avec le fichier d'origine »).
- Ma collection : une image retirée apparaît tout de suite dans « Images retirées », et son compte suit, sans rouvrir la page.

## 0.15.0

- Assistant : sous chaque réponse, « Coût ●●○ » dit si la demande était petite, moyenne ou grosse chez votre service, et un clic en donne le détail (échanges, tokens envoyés, relus en cache, reçus). Rien ne s'affiche avec un service installé sur l'ordinateur (Ollama).
- Assistant : « ↶ Défaire » à côté d'une modification proposée la retire et garde les autres. Si une modification suivante s'appuie dessus, un message dit laquelle défaire d'abord. « ↷ Rétablir » remet la dernière modification défaite. L'assistant est prévenu, à la demande suivante, que des modifications ont été défaites.
- Assistant avec Claude : Cahutin passe par l'API Messages d'Anthropic, qui garde en cache la consigne et la description des outils, renvoyées à chaque échange. Les demandes coûtent moins cher.
- Assistant : la description des outils envoyée à chaque échange est plus courte (11 Ko au lieu de 17), avec tous les services ; les règles communes (identifiants, pages, couleurs, champs de la classe) sont dites une fois dans la consigne.
- Assistant : chaque demande note dans le journal ce qu'elle a coûté (échanges, appels d'outils, tokens envoyés, reçus, lus et écrits en cache), sans la demande ni la réponse. Pour les développeurs, un banc d'essai rejoue une vingtaine de demandes types avec un vrai service.

## 0.14.0

- Assistant : il s'en tient à la mise en page et aux courts textes de la page. Une demande hors de ce cadre (une rédaction, une autre conversation, « oublie tes consignes ») reçoit un refus poli d'une phrase, et un message de plus de 2000 caractères n'est pas envoyé : il reste dans le champ, pour le raccourcir.
- Assistant : moins de données envoyées au service à chaque demande, donc moins chère et plus rapide. Il lit la page affichée plutôt que tout le document, sans les réglages laissés par défaut, et les dispositions déjà lues ne sont pas renvoyées.
- Assistant : des idées de demandes quand le panneau est vide, un bouton « ⟳ Réessayer » après une erreur (coupure, service indisponible) quand rien n'a encore été changé, « ↺ Nouvelle conversation » pour repartir de zéro, et, après une modification, une suite proposée juste au-dessus du champ de saisie, qu'un clic y écrit.

## 0.13.0

- Assistant de mise en page, facultatif et désactivé au départ (Préférences > Assistant) : on écrit ce qu'on veut, l'assistant prépare une proposition, et rien ne change avant « Accepter ». Cahutin ne fournit pas l'IA : il se branche sur votre propre service (Claude, OpenAI, Gemini, en général payants à l'usage, Ollama sur l'ordinateur, ou un autre). Il envoie à ce service la disposition des pages et vos demandes, jamais les textes, les photos ni la liste de la classe ; le détail est dans « Confidentialité » (README et site). Une réponse se signale par e-mail, sans compte (« ⚑ Signaler »), la clé s'efface (« Oublier la clé »), et seule une adresse chiffrée (https) est acceptée, sauf pour un service sur l'ordinateur.
- Aide : un guide « L'assistant IA » (l'activer, ce qui est envoyé, accepter ou refuser, signaler une réponse), une astuce, et la fenêtre des nouveautés de la 0.13, avec ses captures.
- Champs faciles à trouver : un tiroir Champs dans la barre de gauche, sections « Champs de l'élève » (prénom, nom, groupe, âge, anniversaire, une lettre du prénom, les anniversaires d'un mois), « Date » et « La classe », et bouton « { } Champ » sur tout texte choisi pour en ajouter un ou enlever un champ coché. Sur la page, un champ s'affiche en pastille (« Prénom ») ; à l'impression et à l'export PDF, un champ pas encore rempli laisse un blanc (une feuille où l'enfant écrit son prénom). Copié vers un autre logiciel, il garde ses accolades.
- Les champs s'écrivent sans accents ({prenom}, {age}, {PRENOM_1}, {anniversaires_mars}) et ce sont eux seuls qui se remplissent : une zone dont l'invite est « Prénom » ne l'est plus. Les documents et les modèles déjà faits se convertissent seuls à l'ouverture, un texte déjà rempli n'est jamais réécrit ; les modèles fournis utilisent les nouveaux champs. Un document qui en contient ne s'ouvre plus dans une version plus ancienne de Cahutin.
- Champs de la date et de la classe : {date} (« 9 novembre 2026 »), {date_courte}, {date_longue}, {jour_mois}, {jour_semaine}, {mois}, {annee}, {annee_scolaire} (« 2026-2027 »), {saison}, et {classe}, {niveau}, {nombre_eleves}, {ecole}, {enseignant}. Ils s'écrivent une fois et ne changent plus : à la création d'une page depuis un modèle (une petite fenêtre demande la date, ouverte sur celle du jour), dès qu'on les pose dans un document, ou à la fin de l'écriture d'un texte où on les a tapés. En mode modèle, ils restent des pastilles. L'école et l'enseignant se règlent sur la page « Ma classe » ; tant qu'ils sont vides, ou sans classe choisie, le champ reste en pastille et s'écrit dès que l'information arrive.
- Une planche remplie avec moins d'élèves que d'étiquettes, ou pour un enfant sans photo ni symbole, n'est plus acceptée pour un nouveau remplissage.
- Tiroirs Texte, Formes, Décors et Polices rangés en sections qui se replient, repliées au départ, chacune avec une ligne qui dit ce qu'elle contient ; la section (PS, MS, GS) des fiches reste visible.
- Copie de secours : toutes les deux minutes, le document modifié et pas encore enregistré est copié, avec ses photos, dans le dossier de l'application. Si Cahutin s'arrête sans enregistrer (panne, coupure de courant), il propose au lancement suivant de reprendre ce travail : « Reprendre » le rouvre, à enregistrer ; « Non, l'oublier » efface la copie. Elle est aussi effacée à chaque enregistrement et quand on quitte le document.
- Photothèque de la classe : les photos retirées vont dans la corbeille de l'ordinateur, d'où elles se récupèrent, comme un document ou une classe. Le bouton s'appelle « Mettre à la corbeille ».
- Ma classe : retirer un élève ne pose plus de question ; un message « Léa n'est plus dans la classe. » propose « Remettre dans la classe », qui le remet à sa place.
- La question avant de quitter un document modifié le nomme (« Enregistrer « Sortie à la ferme » avant de continuer ? ») ; son bouton de retour s'appelle « Revenir au document » ; Échap y revient aussi, et Entrée enregistre.
- La barre au-dessus de l'élément choisi range les outils moins utilisés (miroir, quarts de tour, frise, répéter, découpe…) dans « ⋯ Plus » : elle est plus courte. Pour tout voir d'un coup d'œil, décocher ce réglage dans les Préférences.
- Plus fluide sur un ordinateur modeste : les petites photos (bande des pages, accueil, pages en vis-à-vis) chargent une image réduite, et les photos restent affichées pendant un enregistrement, un export ou un flou de visages. L'impression et l'export PDF utilisent toujours les photos entières.
- Un document reçu dont les photos et polices dépassent 1 Go n'est pas ouvert, avec le message « Ce document est trop volumineux pour être ouvert. » ; une image de plus de 12 000 pixels de côté est refusée à l'import, avec « image trop grande ».
- Aide > Signaler un problème : après l'enregistrement du rapport, la messagerie de l'ordinateur s'ouvre sur un message déjà adressé (cahutin.pavilion366@slmail.me), qui dit quel fichier joindre ; on raconte ce qui s'est passé, on joint le rapport et on envoie. Sans messagerie, Cahutin donne l'adresse.
- Dans Ma collection, un nom d'image ou de thème ne peut plus contenir de deux-points. L'import d'un document par LibreOffice n'exécute aucune macro.

## 0.12.0

- Rogner une photo : en mode Recadrer (bouton « Recadrer » ou double-clic), des poignées orange sur le cadre rognent ses bords, la photo restant en place ; la partie coupée se voit en transparence autour du cadre. Glisser la photo et la molette marchent comme avant ; chaque geste s'annule séparément.
- Panneau Réglages d'une photo, section « Recadrage » : rognage de chaque bord en mm, « Agrandir dans le cadre » (100 à 500 %), position de gauche à droite et de haut en bas, et « Photo entière », qui retire le rognage (la photo garde sa taille, le cadre s'élargit, sans dépasser la page).
- Agrandir ou réduire une photo par ses poignées garde ses proportions, donc son rognage, son zoom et son recadrage ; avec Maj enfoncée, elle se redimensionne librement. Pour les formes et les textes, rien ne change (Maj garde les proportions).
- Frise posée : dans sa barre, « Modifier la frise » règle la taille des images, leur espacement et la distance au bord (la marge avec la page, ou l'écart avec le bloc qu'elle entoure), et « Changer les images… » met d'autres images à la même place ; « Supprimer la frise » la retire seule, en gardant le bloc qu'elle entourait.
- Bouton « 💾 Enregistrer » à côté du titre du document. Le nom proposé au premier enregistrement est le titre, même tapé à l'instant sans quitter le champ (Ctrl+S proposait alors « Sans titre »).
- Double-clic sur un document .carnet : Cahutin s'ouvre sur ce document. L'installeur Windows (et le paquet du Microsoft Store) associe les fichiers .carnet à Cahutin, et retire l'association à la désinstallation.
- Panneau Réglages rangé en sections qui se replient d'un clic sur leur titre : « Position et taille », « Miroir et ordre », « Recadrage » (photo), « Apparence » (photo, forme), « Mise en forme » (texte). Au départ, Miroir et ordre et Recadrage sont repliés ; chaque section reste ensuite comme on l'a laissée, sur cet ordinateur.
- Préférences plus larges, à la taille de l'écran : sur un grand écran, les réglages se rangent en deux colonnes et tiennent sans défiler.
- Menu Fichier : les exemples sont rangés dans un sous-menu « Exemples ▸ », qui s'ouvre au survol ou au clic.
- La molette de la souris fait défiler la bande des pages, en bas à gauche.
- Zoom comme dans les autres logiciels : Ctrl et la molette (le point sous la souris reste en place), Ctrl et + ou -, Ctrl et 0 pour « Ajuster ».
- Copier un passage d'un texte et le coller dans une autre zone garde sa mise en forme : police, taille, couleur, gras, italique, souligné, même quand elles venaient de la zone d'origine. Un texte copié hors de Cahutin reste collé sans mise en forme.
- Une classe se supprime : bouton « Supprimer la classe… » dans la page Ma classe. Après confirmation, elle part dans la corbeille de l'ordinateur, d'où elle se récupère ; les documents déjà faits pour ses élèves ne changent pas.
- Curseurs des poignées (redimensionner, rogner) : ceux de l'ordinateur, à leur taille habituelle, dans le sens de la poignée même sur un élément tourné, à la place des grosses flèches dessinées. Sous Linux, où certains thèmes n'ont pas les flèches diagonales, une flèche double dessinée les remplace, de la même taille sur un écran avec ou sans mise à l'échelle.
- La poignée de rotation est plus près de l'élément, et la barre au-dessus ne la recouvre plus, même sur un petit élément.
- Barre d'un groupe : une fois le groupe verrouillé, le bouton devient « 🔓 Déverrouiller » (il restait « Verrouiller »). La barre et le panneau Réglages suivent la même règle : « Déverrouiller » quand tout le groupe est verrouillé, « Verrouiller » sinon.
- Accueil, clic droit sur un document récent : « Ouvrir », « Retirer de la liste » et « Supprimer le fichier… » ne faisaient rien. Ils marchent.
- Version Linux : après avoir relâché Maj, le redimensionnement restait libre et la rotation sans crans. Maj relâchée est maintenant bien prise en compte (et oubliée si l'on quitte la fenêtre en la tenant).
- Recadrer en glissant la photo ne l'emporte plus parfois comme une image de navigateur (image fantôme semi-transparente).
- Pendant l'écriture, un passage dans une police dont le nom contient un chiffre seul, un point ou une parenthèse (« Cursive 2 ») s'affichait dans la police de la zone ; il garde maintenant sa police.
- La mise à la corbeille d'un document (ou d'une classe) qui échoue est notée dans le journal de l'application, avec la sorte d'erreur.

## 0.11.0

- Import d'un PDF : un titre en cursive (ou toute police dont les lettres sont posées une à une) arrive en un seul texte, et non plus lettre par lettre ; son cadre est assez haut pour ses boucles, sans être signalé « Texte trop long ».
- Guides (carte « Guides » de l'accueil, menu Aide > Guides, bouton « ? Comment faire » sur place, ou sous une astuce ; la visite guidée est en tête de la liste) : un livret qui se feuillette, une image (le bouton dont on parle entouré) ou, pour un geste, une courte vidéo lente, et deux ou trois phrases par étape ; à la fin, « Me guider » montre le vrai bouton de chaque étape sur l'écran et passe à la suivante quand c'est fait, sans rien bloquer. Huit guides : « Créer un modèle », « Remplir avec ma classe », « Photothèque de la classe », « Flouter des visages », « Étiquettes à découper », « Reprendre une ancienne fiche », « Frise autour d'un bloc » et « Ma collection ». Avec les animations réduites, les vidéos restent fixes.
- Fusionner des cases en glissant ne sélectionne plus le texte « Glissez une photo ici » ; le ✓ ou le ✗ de la fusion se place dans le coin de la case.
- Daltonisme, dans les Préférences, chaque aide activable : les couleurs proposées disent leur nom (« Rouge », « Vert »…) et celui de la couleur choisie s'écrit à côté ; ce qui ne se distinguait que par la couleur reçoit aussi un signe (bouton actif souligné, fusion de cases ✓ ou ✗, zones de texte d'un modèle en pointillés ronds) ; des couleurs que les daltoniens distinguent (palette d'Okabe et Ito), pour des fiches lisibles par un élève daltonien ; un bouton « Vision daltonienne » qui montre la page comme la voit une personne daltonienne, sans la modifier.
- Préférences : animations réduites (par défaut si le système le demande) ; en icônes seules, le nom de l'outil peut apparaître après une demi-seconde ou une seconde.
- Préférences enrichies. Affichage : taille de l'interface (100, 125 ou 150 %), police des menus et des boutons (habituelle, Andika très lisible, OpenDyslexic), contraste élevé (textes noirs, bordures foncées, fonds blancs), grandes poignées pour un écran tactile ou un tableau blanc. Nouvelle page : son format (A5, A4, A3, portrait ou paysage) et la police des nouveaux textes. La section des fiches de graphisme choisie d'avance. Là où le PDF est proposé, l'export qui vient en premier dans « Imprimer / Exporter ». Chaque réglage s'applique tout de suite et reste sur l'ordinateur.
- Un tout petit élément (le point de départ d'une ligne de graphisme, un repère) se déplace sans être redimensionné par erreur : ses poignées s'écartent autour de lui et son centre reste libre pour le saisir.
- Une forme sans fond se choisit et se déplace en cliquant n'importe où à l'intérieur, comme une forme remplie, même posée sur une autre forme : l'absence de fond ne change que l'apparence. Le cadre de page des Décors, les repères de découpe et les tracés de graphisme laissent passer les clics à l'intérieur, vers ce qu'ils entourent ou vers la page.
- Panneau Réglages : les mêmes réglages pour tout, photo, image, forme, texte ou groupe. Position, taille (avec « Garder les proportions », coché pour une photo ou une image), rotation avec quarts de tour, miroir (sauf un texte), devant ou derrière. Un groupe se règle comme un élément : sa taille l'agrandit en entier, à ses proportions.
- Miroir pour tout : un seul outil « Miroir » dans la barre, en deux moitiés côte à côte, ⇋ (gauche-droite) et ⇅ (haut-bas), qui retournent une photo, une image ou une forme (la queue d'une bulle de parole, une flèche, une main qui montre l'autre côté). Sur un groupe, tout le groupe se retourne, les textes restant lisibles. Le miroir passe à l'export LibreOffice. Un document qui retourne autre chose qu'une photo de gauche à droite demande cette version de Cahutin.
- Mode modèle plus simple : chaque texte et chaque image a dans sa barre (et dans Réglages) le choix « À remplir » ou « Fixe ». Un texte du tiroir Texte est à remplir, une image du tiroir Images est fixe (un logo, un dessin) ; la case photo à remplir est dans le tiroir Images. Les boutons « Zone texte » et « Image fixe » disparaissent de la barre de gauche. Un élément sans l'un ni l'autre est signalé : il n'est pas gardé dans le modèle.
- Barre au-dessus de l'élément choisi : tous les outils y sont visibles, et passent à la ligne si l'écran est trop étroit, sans rien cacher. Au choix dans les Préférences, les outils moins utilisés (miroir, quarts de tour, frise, répéter, repères de découpe, ombre et collection pour une photo) se rangent dans un menu « ⋯ Plus » pour une barre plus courte.
- Fenêtre Préférences (bouton « Préférences » en haut) : la barre montre les icônes et les noms, ou, au choix, les icônes seules ; le nom apparaît alors aussitôt au survol, dans une languette de la barre au-dessus de l'icône, sans rien déplacer. Les astuces pendant le travail et l'avis des nouvelles versions s'y règlent aussi.

## 0.10.1

- Version AppImage (Linux) : l'import d'un PDF, d'un document Word ou LibreOffice échouait (« LibreOffice n'a pas pu lire ce fichier ») : LibreOffice héritait des bibliothèques de l'AppImage et ne démarrait pas. Il est désormais lancé avec l'environnement du système.
- Import d'un fichier LibreOffice Draw (.odg) : les photos posées dans une forme (cadre arrondi, polygone) et les images rangées en deux exemplaires (un dessin WMF et sa copie PNG) étaient perdues. Elles sont reprises, chacune tournée dans le sens de son cadre, et les images d'une page sont lues en parallèle : un document de 22 pages et 222 images s'ouvre en une dizaine de secondes.
- Recadrer une photo tournée (ou en miroir) : la photo suit maintenant la souris, au lieu d'aller dans le sens qu'elle avait avant d'être tournée.
- Tourner plus simplement : la poignée de rotation avance par crans de 15° (Maj enfoncée pour tourner librement), et deux boutons de la barre, ↺ et ↻, tournent d'un quart de tour (un groupe autour de son centre).
- Un clic pour choisir une image ne la déplace plus par mégarde : un bloc ne bouge qu'à partir d'un vrai glisser.
- La légende d'une photo tournée se place sous la photo telle qu'elle s'affiche, aussi large qu'elle, au lieu de la recouvrir.

## 0.10.0

- Fiches de graphisme : tiroir Formes > Graphisme. Des lignes de ponts, coupes, vagues, boucles, créneaux, dents de scie, traits ou ronds à repasser, à la taille de la section (PS, MS, GS) : un point vert pour commencer, le modèle en trait plein, la suite en pointillés gris. Et des « Chemins à suivre » en pointillés, à étirer entre deux images. Ces tracés s'impriment et s'exportent pour LibreOffice tels quels ; un document qui en contient demande cette version de Cahutin.
- Labyrinthes et symétrie : tiroir Formes. « Nouveau labyrinthe » en crée un différent à chaque clic (4, 6 ou 9 cases de côté selon la section), avec un poussin à l'entrée et sa maman à la sortie. « Symétrie » pose un quadrillage, l'axe en rouge et la moitié d'un dessin (sapin, maison, fusée, bateau, cœur) à compléter. « Repères pour écrire » : un point vert de départ et une flèche verte de sens, à poser sur les lettres à repasser.
- Images à colorier : dans les pictogrammes du tiroir Images, « À colorier » pose les images en contour noir seul (toutes les images d'OpenMoji), pour les coloriages et les contours à repasser. Une image déjà posée bascule avec « À colorier » dans sa barre flottante. Un document qui en contient demande cette version de Cahutin.
- Import d'un PDF : les tracés (graphisme, cadres arrondis) deviennent des formes modifiables, qu'on peut recolorer, déplacer ou supprimer une à une ; au-delà de 60 tracés sur une page (une illustration), ils restent une image fixe, plus fidèle.
- Les dessins SVG (le format le plus courant des dessins libres à télécharger) s'importent comme des photos, avec leur transparence.
- Tiroir Images : le tiroir Photos devient Images, avec trois rubriques. Photos (celles de ce document, et la photothèque de la classe), Collection (vos dessins, gardés pour tous vos documents) et Pictogrammes, qui quittent le tiroir Décors. Une recherche cherche à la fois dans la Collection et les pictogrammes. « Remplacer » ouvre la bonne rubrique. Un message dans Décors le signale aux habitués.
- Collection : dans le tiroir Images, la Collection garde vos propres images (dessins téléchargés, cliparts, images faites à la main) pour tous vos documents. « Ajouter des images » les choisit sur l'ordinateur ; un clic en pose une, ou on la glisse sur la page. Une image retirée de la liste n'est pas effacée : elle est rangée dans le sous-dossier Retirées. Elles sont rangées dans Documents/Cahutin/Images ; une image posée est copiée dans le document, qui reste complet si on l'envoie à une collègue ou si l'image est retirée de la Collection.
- Page « Ma collection », depuis l'accueil ou le tiroir Images (« Organiser ma collection ») : choisir des images d'un clic et les ranger dans un thème (« Automne », « Animaux »… une image peut en avoir plusieurs), renommer une image en cliquant son nom, retirer des images et remettre celles retirées, ajouter un dossier entier. Les thèmes servent de filtres dans le tiroir Images, et la recherche les trouve aussi.
- « ⭐ Ajouter à ma collection », dans la barre d'une photo posée : l'image (un dessin importé, le logo d'un document Word repris) se retrouve dans tous les documents. Une photo de la photothèque où des enfants sont notés, ou un portrait de la liste de classe, n'y va jamais : elle reste avec ses autorisations.
- Tiroir Décors : cadres et frises seulement ; « Choisir les images » d'une frise ouvre une fenêtre où l'on choisit et numérote les motifs.
- Frises : les motifs se choisissent parmi les pictogrammes et les images de votre collection, ramenées à la taille d'un pictogramme sans être déformées. Deux réglages dans Décors avant de poser : taille des images (15 mm par défaut) et espacement. Une frise déjà posée se règle après coup : sélectionnez-la, puis « Frise » dans sa barre ; elle est refaite à la même place, avec les mêmes motifs dans le même ordre.
- Groupes dans des groupes : grouper des éléments déjà groupés (une frise et une photo avec sa légende) les garde entiers dans le nouveau groupe, et « Dissocier » n'enlève plus qu'un niveau : on retrouve la frise et la photo avec sa légende, chacune toujours groupée. Un double-clic entre d'un niveau à la fois (puis modifie l'élément seul), Échap remonte d'un niveau. Une frise posée autour d'un bloc forme un groupe à elle dans ce groupe. Ces niveaux sont gardés à l'export LibreOffice ; un document qui en contient demande cette version de Cahutin.
- Documents récents (accueil) : une croix retire un document de la liste, « Vider la liste » les retire tous ; le fichier reste où il est. Un clic droit sur un document propose aussi « Supprimer le fichier… », qui le met à la corbeille de l'ordinateur, d'où on peut le récupérer.
- Accueil : « Ma collection » et « Ma classe » ont leur propre rangée, sous les choix pour faire un document.
- Photos (tiroir Images) : une croix en haut à droite de chaque photo la retire du document, après confirmation. Si elle est posée sur une page, sa case reste, vide ; le fichier sur l'ordinateur n'est pas touché, et « Annuler » la remet.
- Une photo posée deux fois de suite d'un clic se décale un peu au lieu de se cacher sous la première. La barre d'une photo n'affiche plus deux boutons « Remplacer ».
- Photos : un clic pose une photo de plus, à ses proportions ; il ne remplace plus la photo choisie (ce qui recadrait la nouvelle dans l'ancien cadre). Une case photo de modèle se remplit toujours d'un clic, et « Remplacer » dans la barre d'une photo la change.
- Un clic sur un pictogramme pose toujours une nouvelle image : il ne remplace plus l'image choisie par mégarde. Pour changer une image, « Remplacer » dans sa barre flottante.
- Un texte ajouté depuis le tiroir Texte (« Titre »…) s'ouvre avec son texte modèle sélectionné : on tape directement le sien.
- Deux éléments ajoutés de suite ne se posent plus exactement l'un sur l'autre.
- Tiroir Formes : le champ « Lignes » du graphisme ne déborde plus du tiroir.
- Un tiroir n'écrase plus ses boutons sous le texte qui les suit quand la fenêtre est haute (tiroir Formes).

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
