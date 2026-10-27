# Recette Windows du prototype

À dérouler sur un poste Windows 11. Consigner les résultats dans `docs/recette/<date>-windows.md`, une ligne par point : OK, ou le problème constaté.

## 1. Construire l'installeur

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
5. Utiliser « Exporter en PDF », puis « Imprimer » vers « Microsoft Print to PDF ». Attendu : rendu identique à l'écran, aucune zone vide ni texte d'invite visible.
6. Créer un modèle personnel, copier son fichier `.carnet-modele` sur un autre poste, dans `%APPDATA%\fr.carnet.app\modeles`, et l'utiliser.
7. Modifier un document, puis fermer la fenêtre. Attendu : l'application demande « Enregistrer les modifications ? ».
8. Utiliser le bouton « ? » (rapport de problème). Attendu : le fichier ne contient ni chemin, ni texte de document, ni nom d'enfant.
9. **Test des 5 minutes**, avec au moins un collègue qui découvre l'outil. Chronométrer, depuis le lancement, la création d'une page avec 4 photos et 2 textes, exportée en PDF. Noter les hésitations.
