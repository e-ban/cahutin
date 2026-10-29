# Recette Windows du prototype

À dérouler sur un poste Windows 11. Consigner les résultats dans `docs/recette/<date>-windows.md`, une ligne par point : OK, ou le problème constaté.

## 1. Construire l'installeur

**Le plus simple :** télécharger l'installeur `Cahutin_<version>_x64-setup.exe` de la dernière version sur la page « Releases » du dépôt. Pour essayer une construction plus récente : à chaque envoi sur `main`, GitHub construit l'installeur (workflow « Construction ») ; ouvrir « Actions », puis le dernier run réussi « Construction », et télécharger l'artefact `carnet-installeur-windows-…`. On peut aussi relancer la construction à la main avec « Run workflow ».

**Sur le poste Windows lui-même :**

1. Installer :
   - rustup, avec la chaîne `stable-x86_64-pc-windows-msvc` ;
   - Node 26 ;
   - les Build Tools de Visual Studio (charge de travail « Développement Desktop en C++ »).

   WebView2 est déjà présent sur Windows 11.
2. Dans le dossier du projet : `npm ci`, puis `npm run tauri build`.
3. Résultat attendu : `target\release\bundle\nsis\Cahutin_<version>_x64-setup.exe`.

## 2. Point de contrôle (en développement)

Avec `npm run tauri dev` :

- [ ] Les photos importées s'affichent : elles sont servies par `http://carnet-asset.localhost/...`.
- [ ] Les polices embarquées s'affichent : le titre en cursive est bien en Playwrite FR Moderne.
- [ ] « Imprimer » ouvre la boîte d'impression avec les pages seules.
- [ ] Glisser une vignette sur une zone photo y place la photo.

## 3. Fidélité du texte sous Windows

1. Dans l'application lancée par `npm run tauri dev`, ouvrir la console de développement (clic droit, Inspecter), puis exécuter `location.href = "/fidelity.html"`.
2. Comparer le résultat affiché aux valeurs `expected_lines` de `fixtures/text_fidelity.json`.
3. S'ils diffèrent, mettre le fichier à jour et le noter dans le compte rendu.
4. Avec LibreOffice et poppler pour Windows dans le `PATH`, lancer : `cargo test -p carnet-core --test odg_libreoffice -- --ignored --test-threads=1`. Résultat attendu : 19 tests réussis. Puis le banc d'import : `cargo test -p carnet-core --test import_libreoffice -- --ignored --test-threads=1`, 3 tests réussis.

## 4. Recette de l'installeur

1. Installer avec un **compte standard** : aucune demande d'administrateur ne doit apparaître. Windows affiche l'avertissement SmartScreen « Windows a protégé votre ordinateur » : cliquer sur « Informations complémentaires », puis sur « Exécuter quand même ».
2. **Couper le réseau** pour toute la suite.
3. Importer un dossier de photos de téléphone contenant des photos en portrait, en paysage et une HEIC. Attendu : les photos sont droites et la HEIC est signalée par un message clair.
4. Créer une page depuis « Grille de photos », avec 4 photos et 2 textes. L'exporter pour LibreOffice, puis ouvrir le fichier `.odg` sur un poste **sans** les polices installées. Attendu : même apparence, objets modifiables.
5. Utiliser « Exporter en PDF », puis « Imprimer » vers « Microsoft Print to PDF ». Attendu : rendu identique à l'écran, aucune zone vide ni texte d'invite visible. Recommencer avec un document dont la première page est en paysage : vérifier que la page du PDF est bien A4 paysage, sans être tournée deux fois. Un document qui mélange portrait et paysage ne prend qu'une orientation à l'export PDF ; il passe par « Imprimer ».
6. Créer un modèle personnel, copier son fichier `.carnet-modele` sur un autre poste, dans `%APPDATA%\fr.cahutin.app\modeles`, et l'utiliser.
7. Modifier un document, puis fermer la fenêtre. Attendu : l'application demande « Enregistrer les modifications ? ». Recommencer en fermant la fenêtre **pendant la saisie** d'un texte : la question doit aussi apparaître, et le texte tapé doit être enregistré.
8. Utiliser Aide > Signaler un problème. Attendu : le fichier ne contient ni chemin, ni texte de document, ni nom d'enfant.
9. **150 photos sans figer** : dans le tiroir Photos, « 📁 Dossier » sur un dossier d'environ 150 photos de téléphone. Attendu : la barre « Import en cours : n / 150 » avance, les vignettes apparaissent au fil de l'eau, et pendant l'import un clic sur une autre icône de la colonne (Texte, Formes) ouvre son tiroir tout de suite. Le défilement des vignettes reste fluide. Enregistrer avec 4 photos placées, fermer, rouvrir le document : les 146 autres reviennent dans « Non utilisées ».
10. **Cache des photos** : après la fermeture de l'application, le dossier `%LOCALAPPDATA%\fr.cahutin.app\images` est vide (ou absent).
11. **Ajouter une police, l'exporter** : tiroir Polices, « ＋ Ajouter une police », choisir un `.ttf` libre (par exemple une police OFL téléchargée **avant** de couper le réseau). Écrire un texte dans cette police, l'exporter pour LibreOffice, puis ouvrir l'ODG sur un autre poste où la police n'est pas installée. Attendu : le texte garde la police. Ouvrir aussi le `.carnet` sur cet autre poste : Cahutin propose « Ce document utilise la police X : l'ajouter ? ».
12. **Police à licence restreinte** : ajouter une police dont la licence interdit l'intégration (de nombreuses polices commerciales installées avec Windows sont dans ce cas). Attendu : la mention « ⚠ export » et sa phrase d'explication dans le tiroir ; l'ODG ne l'emporte pas.
13. **Test des 5 minutes, à refaire après la refonte**, avec au moins un collègue qui découvre l'outil. Chronométrer, depuis le lancement (écran d'accueil), la création d'une page avec 4 photos et 2 textes, exportée en PDF. Noter les hésitations, en particulier : trouve-t-il la barre flottante, le clic droit, le tiroir Photos ?
14. **Grouper, agrandir, dissocier** : sélectionner au cadre trois images et un texte, Ctrl+G. Attendu : un clic sélectionne tout le groupe ; agrandir par un coin agrandit aussi le texte, sans déformer les images ; double-clic sur le texte pour l'écrire sans dissocier, Échap revient au groupe ; Ctrl+Maj+G dissocie. L'ODG ouvert dans LibreOffice montre un groupe (menu Forme > Groupe > Dissocier disponible).
15. **Frise et cadre** : tiroir Décors, cadre « Double », puis « Choisir les images » dans Frises : cliquer un renard, une étoile et un cœur (aucune image n'est posée sur la page pendant ce choix), « Terminé », puis « Tout autour ». Attendu : une frise régulière qui répète renard, étoile, cœur sans interruption tout autour, sans doublon aux coins ; elle se sélectionne et se supprime d'un coup ; frise sélectionnée, cliquer une autre image change toute la frise. Les icônes d'alignement du texte ressemblent à celles d'un traitement de texte.
16. **Images OpenMoji et mention** : rechercher « gâteau », poser l'image. Attendu : la ligne « Pictogrammes : OpenMoji, CC BY-SA 4.0 » en bas de la page, à l'écran, à l'impression, dans le PDF et dans l'ODG ; l'écran Aide > À propos cite OpenMoji.
17. **Document avec images ouvert par l'ancienne version** : ouvrir ce `.carnet` avec une version antérieure de Carnet. Attendu : le message « créé par une version plus récente » (et non « fichier abîmé »).
18. **Légende** : sélectionner une photo, bouton « Légende ». Attendu : un texte sous la photo, prêt à écrire ; déplacer la photo déplace la légende ; recliquer « Légende » sélectionne la légende existante.
19. **Fonds de page** : bouton « Fond » de la bande des pages, puis Seyès, petits carreaux, lignes, pointillés et papier kraft ; « Appliquer à toutes les pages ». Attendu : le fond ne se sélectionne jamais ; il est identique à l'écran, à l'impression, dans le PDF et dans l'ODG (dans LibreOffice, il ne se sélectionne pas non plus) ; une page en paysage est couverte jusqu'aux bords.
20. **Exemples** : écran d'accueil, « Découvrir un exemple », ouvrir chacun des cinq. Attendu : une copie sans nom (« Enregistrer » demande un nom), photos à déposer dans les zones ; si le document en cours est modifié, Cahutin demande d'abord d'enregistrer.
21. **Alignement vertical et marge** : un texte de deux mots dans un grand cadre ; menu d'alignement de la barre flottante, rangée du bas : en haut, au milieu, en bas. Réglages : marge intérieure à 6 mm. Attendu : même position à l'écran, à l'impression et dans l'ODG ouvert dans LibreOffice ; un texte trop long reste lisible depuis le haut et affiche « Texte trop long ».
22. **Cadre de texte** : bouton « Cadre » d'un texte : fond jaune pâle, bordure de 1 mm en tirets, coins de 8 mm. Attendu : identique dans LibreOffice, où le texte se modifie d'un double-clic ; un texte vide encadré (étiquette) garde son cadre à l'export.
23. **Bordures en tirets et pointillés** : une photo avec bordure en pointillés, une forme et un trait en tirets. Attendu : mêmes tirets à l'écran, à l'impression et dans LibreOffice.
24. **Frise autour d'un bloc** : dans Décors, choisir deux images comme motifs ; sélectionner une photo, puis « Frise » dans la barre flottante. Attendu : la frise entoure la photo, à cheval sur son bord ; elle se déplace, s'agrandit et tourne avec la photo ; Ctrl+Maj+G la dissocie.
25. **Image de fond d'une étiquette** : importer une image de cadre d'étiquette dans Photos ; sélectionner un texte, « Cadre », puis cette image sous « Image de fond ». Attendu : l'image couvre le cadre, sous le texte ; enregistrer comme modèle, puis créer une page depuis ce modèle : l'étiquette garde son image. Ouvrir ce document avec Carnet 0.4 : message « version plus récente » (et non perte du cadre).
26. **Accrochage au centre** : déplacer une photo vers le milieu de la page, puis vers le centre d'un autre bloc. Attendu : une ligne bleue apparaît et le bloc s'arrête exactement au milieu (Réglages : X = 75,0 pour un bloc de 60 mm de large). Recommencer avec un groupe et avec deux blocs sélectionnés à la fois.
27. **Nouveaux modèles** : créer une page depuis chacun des modèles « Étiquettes », « Étiquettes photo », « Photos et textes », « Photos légendées » et « Affiche paysage ». Écrire des prénoms et déposer des photos dans les étiquettes, puis imprimer la planche « Étiquettes ». Attendu : aucune étiquette n'est coupée par l'imprimante ; l'affiche est en paysage ; déplacer une étiquette photo déplace sa photo et son prénom.
28. **Plusieurs polices dans une étiquette** : dans une étiquette, écrire le prénom sur trois lignes ; sélectionner la deuxième ligne et choisir OpenDyslexic dans la barre flottante, puis la troisième et choisir Playwrite FR Moderne en 32. Attendu : l'écriture continue après chaque choix, seule la ligne sélectionnée change ; la barre affiche la police de la ligne où se trouve le curseur ; même rendu à l'impression et dans LibreOffice.
29. **Importer des fichiers** : Fichier > Importer…, puis choisir ensemble une fiche Word (.docx) et une fiche Publisher (.pub) de collègues, et un document `.carnet`. Attendu :
    - une bande « Import de … en cours… » avec Annuler ;
    - les pages arrivent après la page courante, une seule annulation (Ctrl+Z) les retire toutes ;
    - les textes se modifient, avec les lignes d'un paragraphe en un seul texte ;
    - le compte rendu donne, pour chaque fichier, les pages, textes, images et formes repris, et les dessins non repris.

    Recommencer depuis l'écran d'accueil (« Importer un fichier ») : un document neuf avec ces seules pages. Pendant l'import d'un gros PDF (plus de 20 pages), cliquer Annuler : la conversion s'arrête, le Gestionnaire des tâches ne montre plus de `soffice.bin`, et `%TEMP%` ne contient plus de dossier `carnet-import…`. Désinstaller (ou renommer) LibreOffice : l'import d'un .docx affiche « Pour importer ce fichier, installez LibreOffice (gratuit), puis recommencez… », celui d'un `.carnet` marche toujours.
30. **Visite guidée et astuces** : sur un poste neuf (ou après avoir supprimé les données de Cahutin), lancer Cahutin et fermer l'écran d'accueil. Attendu : la visite démarre ; chaque étape met en lumière la bonne zone ; Échap la ferme ; elle ne revient pas au lancement suivant. Pendant le test des 5 minutes, noter si la collègue suit la visite jusqu'au bout et ce qu'elle en retient. Menu Aide : Astuces ouvre le panneau ; décocher « Afficher les astuces pendant le travail » fait disparaître les bulles ; Visite guidée la relance.
