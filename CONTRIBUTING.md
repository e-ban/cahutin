# Contribuer à Cahutin

Merci de votre intérêt. Cahutin est fait pour des enseignants qui n'ont pas de temps à perdre avec un logiciel : chaque contribution se juge à cette aune.

En participant, vous acceptez le [code de conduite](CODE_OF_CONDUCT.md).

## Signaler un problème

1. Dans Cahutin, menu Aide > « Signaler un problème… » : il enregistre un rapport (le journal de l'application, sans texte de document, ni nom de fichier, ni prénom).
2. Ouvrir une [issue « Problème »](https://github.com/e-ban/cahutin/issues/new/choose) et y joindre ce rapport.

**Ne joignez jamais de photo d'enfant ni de document avec de vrais prénoms.** Si un document est nécessaire pour reproduire le problème, refaites-le avec des prénoms et des images d'exemple.

Une faille de sécurité ne se signale pas par une issue : voir [SECURITY.md](SECURITY.md).

## Proposer une idée

Ouvrir une [issue « Idée »](https://github.com/e-ban/cahutin/issues/new/choose) : décrire ce que vous voulez faire en classe, plutôt que la solution. La [feuille de route](docs/feuille-de-route.md) dit ce qui est déjà prévu.

## Contribuer du code

Pour un changement important, ouvrez d'abord une issue pour en parler : cela évite de travailler sur quelque chose qui ne serait pas retenu.

L'installation et les commandes sont dans la section « Développement » du [README](README.md). Travaillez sur une branche, puis ouvrez une pull request vers `main`.

### Les règles du projet

- **Hors ligne.** Aucune nouvelle connexion réseau. L'interface ne charge rien de distant (la politique de sécurité de `src-tauri/tauri.conf.json` le garantit, et un test la vérifie).
- **Vie privée.** Le journal ne contient ni texte de document, ni nom de fichier, ni prénom : seulement des nombres et des messages fixes. Les photos perdent leurs métadonnées à l'import.
- **Simple.** Une fonction qui demande une explication pour être trouvée est à revoir. Les textes de l'interface sont en français, courts, sans jargon, et sans tiret long.
- **Licences.** Les dépendances doivent être compatibles avec l'EUPL 1.2 (`cargo deny check licenses`). Une image, une police ou un modèle ajouté indique sa licence et sa source, et figure dans les « Mentions » du README.
- **Format des documents.** Un changement du format `.carnet` monte `FORMAT_VERSION` et garde l'ouverture des anciens documents.

### Avant d'ouvrir la pull request

```sh
npm test                  # tests de l'interface
cargo test --workspace    # tests du moteur et de l'application
npx svelte-check          # pas de nouvelle erreur
cargo deny check licenses # si une dépendance change
```

- Ajoutez une ligne sous `## Non publié` dans [CHANGELOG.md](CHANGELOG.md), écrite pour les enseignants.
- Si le changement se voit, mettez à jour la documentation (README), l'aide de l'application (visite guidée, astuces, bulles d'aide) et, si besoin, le site (`site/`). Joignez une capture d'écran.
- Les messages de commit suivent le style de l'historique, en anglais : `feat(ui): …`, `fix: …`, `docs: …`.

## Publier une version

Réservé au mainteneur : voir « Publier une version » dans le [README](README.md).

## Licence

Vos contributions sont publiées sous la [Licence publique de l'Union européenne (EUPL) 1.2](LICENSE), comme le reste de Cahutin.
