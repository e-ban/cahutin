# Sécurité

Cahutin garde sur l'ordinateur des photos et des prénoms d'enfants. Une faille qui les exposerait est prise au sérieux.

## Versions suivies

Seule la dernière version publiée reçoit des corrections. Cahutin est en bêta : la correction d'une faille sort dans une nouvelle version, à installer par-dessus l'ancienne.

## Signaler une faille

**Ne pas ouvrir d'issue publique.** Passer par un [signalement privé sur GitHub](https://github.com/e-ban/cahutin/security/advisories/new) (onglet Security, « Report a vulnerability »), visible seulement du mainteneur.

Indiquer :

- la version de Cahutin et le système (Windows 11, distribution Linux) ;
- ce qui se passe, et comment le reproduire ;
- si possible, un fichier qui le montre (`.carnet`, `.carnet-modele`, `.cahutin-classe`, document importé). **Jamais de vraie photo d'enfant ni de vrai prénom** : un document d'essai suffit.

Le projet est tenu par une seule personne, sur son temps libre : la réponse vient dès que possible. Une fois la correction publiée, l'avis de sécurité est rendu public, avec remerciements si vous le souhaitez.

## Ce qui compte comme une faille

- Un fichier piégé (`.carnet`, `.carnet-modele`, `.cahutin-classe`, ou document importé) qui fait exécuter du code, lire ou écrire hors des dossiers prévus, ou planter Cahutin de façon répétable.
- Une donnée qui sort de l'ordinateur : connexion réseau autre que la vérification facultative des mises à jour, contenu distant chargé par l'interface.
- Une donnée personnelle qui reste là où elle ne devrait pas : texte, nom de fichier ou prénom dans le journal ou le rapport de problème ; métadonnées de photo (lieu, appareil) gardées à l'import ; photo d'origine encore présente après le flou des visages.
- Un paquet publié dont l'empreinte ne correspond pas à `SHA256SUMS`, ou une signature qui ne se vérifie pas (voir « Vérifier un téléchargement » dans le [README](README.md)).

## Hors du champ

- L'avertissement de Windows sur l'installeur pas encore signé (connu, voir la [politique de signature](https://e-ban.github.io/cahutin/signature.html)).
- Une attaque qui suppose un ordinateur déjà compromis ou un accès administrateur.
- Les failles de LibreOffice, utilisé pour l'import de documents : à signaler au [projet LibreOffice](https://www.libreoffice.org/about-us/security/).
- Le serveur de développement (`npm run tauri dev`), qui ne fait pas partie de l'application installée.
