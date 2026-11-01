# Fiche du Microsoft Store

Dossier à importer dans l'Espace partenaires : `fiche/` (Cahutin > Soumission > Fiches du Store > Importer des fiches). Il ne contient que le CSV et les images : un autre fichier peut faire échouer l'import.

- `fiche/listingData.csv` : la fiche en français (fr-fr), exactement au format de l'export de l'Espace partenaires : UTF-8 avec BOM, lignes terminées par CRLF (LF à l'intérieur d'un texte), pas de fin de ligne après la dernière. Les captures 1 à 5 sont celles déjà en ligne (adresses de l'Espace partenaires) ; les suivantes sont des chemins qui commencent par le nom du dossier importé (`fiche/captures/guides.png`) : Partner Center l'exige. Si l'import échoue, son lien « Afficher les erreurs » dit quel champ est en cause.
- `fiche/captures/` : captures d'écran de l'application, 1896 × 1032, faites sur les exemples fournis avec Cahutin.

Pour une nouvelle version : exporter la fiche de la nouvelle soumission (les adresses des captures en ligne lui sont propres), reporter ici les champs changés (notes de version, description, fonctionnalités, captures), puis importer le dossier.
