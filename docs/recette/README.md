# Recette Windows du prototype

À dérouler sur un poste Windows 11. Consigner les résultats dans `docs/recette/<date>-windows.md`, une ligne par point : OK, ou le problème constaté.

## 1. Construire l'installeur

**Le plus simple :** à chaque envoi sur `main`, GitHub construit l'installeur (workflow « Windows »). Sur la page du dépôt, ouvrir « Actions », puis le dernier run réussi « Windows », et télécharger l'artefact `carnet-installeur-windows-…`. Il contient `Carnet_0.1.0_x64-setup.exe`. On peut aussi relancer la construction à la main avec « Run workflow ».

**Sur le poste Windows lui-même :**

1. Installer :
   - rustup, avec la chaîne `stable-x86_64-pc-windows-msvc` ;
   - Node 26 ;
   - les Build Tools de Visual Studio (charge de travail « Développement Desktop en C++ »).

   WebView2 est déjà présent sur Windows 11.
2. Dans le dossier du projet : `npm ci`, puis `npm run tauri build`.
3. Résultat attendu : `target\release\bundle\nsis\Carnet_0.1.0_x64-setup.exe`.

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
4. Avec LibreOffice et poppler pour Windows dans le `PATH`, lancer : `cargo test -p carnet-core --test odg_libreoffice -- --ignored --test-threads=1`. Résultat attendu : 5 tests réussis.

## 4. Recette de l'installeur

1. Installer avec un **compte standard** : aucune demande d'administrateur ne doit apparaître. Windows affiche l'avertissement SmartScreen « Windows a protégé votre ordinateur » : cliquer sur « Informations complémentaires », puis sur « Exécuter quand même ».
2. **Couper le réseau** pour toute la suite.
3. Importer un dossier de photos de téléphone contenant des photos en portrait, en paysage et une HEIC. Attendu : les photos sont droites et la HEIC est signalée par un message clair.
4. Créer une page depuis « Grille de photos », avec 4 photos et 2 textes. L'exporter pour LibreOffice, puis ouvrir le fichier `.odg` sur un poste **sans** les polices installées. Attendu : même apparence, objets modifiables.
5. Utiliser « Exporter en PDF », puis « Imprimer » vers « Microsoft Print to PDF ». Attendu : rendu identique à l'écran, aucune zone vide ni texte d'invite visible. Recommencer avec un document dont la première page est en paysage : vérifier que la page du PDF est bien A4 paysage, sans être tournée deux fois. Un document qui mélange portrait et paysage ne prend qu'une orientation à l'export PDF ; il passe par « Imprimer ».
6. Créer un modèle personnel, copier son fichier `.carnet-modele` sur un autre poste, dans `%APPDATA%\fr.carnet.app\modeles`, et l'utiliser.
7. Modifier un document, puis fermer la fenêtre. Attendu : l'application demande « Enregistrer les modifications ? ». Recommencer en fermant la fenêtre **pendant la saisie** d'un texte : la question doit aussi apparaître, et le texte tapé doit être enregistré.
8. Utiliser Fichier > Signaler un problème. Attendu : le fichier ne contient ni chemin, ni texte de document, ni nom d'enfant.
9. **150 photos sans figer** : dans le tiroir Photos, « 📁 Dossier » sur un dossier d'environ 150 photos de téléphone. Attendu : la barre « Import en cours : n / 150 » avance, les vignettes apparaissent au fil de l'eau, et pendant l'import un clic sur une autre icône de la colonne (Texte, Formes) ouvre son tiroir tout de suite. Le défilement des vignettes reste fluide. Enregistrer avec 4 photos placées, fermer, rouvrir le document : les 146 autres reviennent dans « Non utilisées ».
10. **Cache des photos** : après la fermeture de l'application, le dossier `%LOCALAPPDATA%\fr.carnet.app\images` est vide (ou absent).
11. **Ajouter une police, l'exporter** : tiroir Polices, « ＋ Ajouter une police », choisir un `.ttf` libre (par exemple une police OFL téléchargée **avant** de couper le réseau). Écrire un texte dans cette police, l'exporter pour LibreOffice, puis ouvrir l'ODG sur un autre poste où la police n'est pas installée. Attendu : le texte garde la police. Ouvrir aussi le `.carnet` sur cet autre poste : Carnet propose « Ce document utilise la police X : l'ajouter ? ».
12. **Police à licence restreinte** : ajouter une police dont la licence interdit l'intégration (de nombreuses polices commerciales installées avec Windows sont dans ce cas). Attendu : la mention « ⚠ export » et sa phrase d'explication dans le tiroir ; l'ODG ne l'emporte pas.
13. **Test des 5 minutes, à refaire après la refonte**, avec au moins un collègue qui découvre l'outil. Chronométrer, depuis le lancement (écran d'accueil), la création d'une page avec 4 photos et 2 textes, exportée en PDF. Noter les hésitations, en particulier : trouve-t-il la barre flottante, le clic droit, le tiroir Photos ?
14. **Grouper, agrandir, dissocier** : sélectionner au cadre trois images et un texte, Ctrl+G. Attendu : un clic sélectionne tout le groupe ; agrandir par un coin agrandit aussi le texte, sans déformer les images ; double-clic sur le texte pour l'écrire sans dissocier, Échap revient au groupe ; Ctrl+Maj+G dissocie. L'ODG ouvert dans LibreOffice montre un groupe (menu Forme > Groupe > Dissocier disponible).
15. **Frise et cadre** : tiroir Décors, cadre « Double », puis une image (par exemple un renard) et « Tout autour ». Attendu : une frise régulière sans doublon aux coins, qui se sélectionne et se supprime d'un coup ; cliquer une autre image change toute la frise.
16. **Images OpenMoji et mention** : rechercher « gâteau », poser l'image. Attendu : la ligne « Pictogrammes : OpenMoji, CC BY-SA 4.0 » en bas de la page, à l'écran, à l'impression, dans le PDF et dans l'ODG ; l'écran Fichier > À propos cite OpenMoji.
17. **Document avec images ouvert par l'ancienne version** : ouvrir ce `.carnet` avec une version antérieure de Carnet. Attendu : le message « créé par une version plus récente » (et non « fichier abîmé »).
18. **Légende** : sélectionner une photo, bouton « Légende ». Attendu : un texte sous la photo, prêt à écrire ; déplacer la photo déplace la légende ; recliquer « Légende » sélectionne la légende existante.
19. **Fonds de page** : bouton « Fond » de la bande des pages, puis Seyès, petits carreaux, lignes, pointillés et papier kraft ; « Appliquer à toutes les pages ». Attendu : le fond ne se sélectionne jamais ; il est identique à l'écran, à l'impression, dans le PDF et dans l'ODG (dans LibreOffice, il ne se sélectionne pas non plus) ; une page en paysage est couverte jusqu'aux bords.
20. **Exemples** : écran d'accueil, « Découvrir un exemple », ouvrir chacun des cinq. Attendu : une copie sans nom (« Enregistrer » demande un nom), photos à déposer dans les zones ; si le document en cours est modifié, Carnet demande d'abord d'enregistrer.
