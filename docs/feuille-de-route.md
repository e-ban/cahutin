# Feuille de route de Cahutin

Document de travail (octobre 2026) : ce que Cahutin pourrait devenir à terme. Rien n'est engagé ; l'ordre des étapes est une proposition, à revoir avec les retours des enseignants.

## 1. Ce que produit une classe de maternelle

Recensement des documents qu'un enseignant de cycle 1 fabrique au long de l'année (sources en fin de document).

| Famille | Documents | Ce qu'ils ont en commun |
|---|---|---|
| **Suivi des apprentissages** | Carnet de suivi (obligatoire, forme libre décidée par l'équipe de cycle), cahier de réussites / de progrès (cases illustrées, une case vide = un progrès à venir), étiquettes-réussite à coller, synthèse des acquis de fin de GS (modèle national, arrêté du 31/12/2015) | Une page **par enfant**, alimentée toute l'année, par domaine du programme ; photos de l'enfant en activité ; transmis aux familles plusieurs fois par an |
| **Liaison avec les familles** | Cahier de vie (photos, dictées à l'adulte, textes de la classe), mots aux parents, invitations | Une page **collective** dupliquée pour chaque enfant, parfois personnalisée (photo de l'enfant, son prénom) |
| **Étiquettes** | Prénoms (porte-manteaux, casiers, tables, dortoir, verres, cahiers), en 3 écritures (capitales, script, cursive), avec photo en PS, étiquettes d'ateliers et de groupes | La **liste de la classe** × un gabarit × une planche à découper ; l'écriture évolue dans l'année (capitales puis script puis cursive) |
| **Affichages et rituels** | Tableau des présences, des responsabilités, emploi du temps visuel avec pictogrammes, date et météo, anniversaires, affiche des groupes, pyramide des âges | Grand format, souvent plastifié, réutilisé d'une année sur l'autre avec une nouvelle liste |
| **Jeux et outils d'écriture** | Memory et loto des prénoms (photo + prénom), lettres mobiles, puzzle prénom, tri des prénoms par longueur, modèles d'écriture du prénom (lignes Seyès, cursive) | Générés depuis la liste de la classe, en plusieurs exemplaires |
| **Fiches d'activité** | Graphisme, numération, tri, coloriage, fiches à compléter | Le cœur actuel de Cahutin |

Constat : **la liste des élèves et leurs photos sont le fil conducteur de presque tout ce qui n'est pas une fiche**. C'est aussi ce qu'aucun outil libre et hors ligne ne fait bien aujourd'hui : les modèles en ligne sont des fichiers Word à remplir à la main, Canva est en ligne, les applications de suivi (Je Valide, Klassroom, Ma Classe de Maternelle...) sont sur tablette, souvent payantes ou hébergées.

Positionnement proposé : **Cahutin reste un outil de fabrication de documents, hors ligne, sur le poste de l'enseignant**, et s'élargit vers la classe (élèves, photos, années) sans devenir un ENT ni une messagerie avec les familles.

## 2. Principes à garder

- **Hors ligne et local** : la liste des élèves et les photos ne quittent jamais l'ordinateur ; jamais dans le journal ni le rapport de problème (déjà la règle pour les documents).
- **Protection des données** : les photos d'enfants sont des données personnelles (RGPD) et relèvent du droit à l'image ; l'autorisation des familles peut être retirée. Cahutin doit aider à respecter cela, pas le compliquer.
- **Pas de compte, pas de serveur.** Le partage entre collègues passe par des fichiers.
- **Simple d'abord** : chaque nouvelle fonction doit pouvoir s'ignorer ; un enseignant qui ne veut faire qu'une fiche ne voit pas la gestion de classe.

## 3. Feuille de route

Chaque étape apporte quelque chose d'utilisable seul. Taille estimée : S (quelques jours), M (une à deux semaines), L (plus).

### Étape A : compléter l'éditeur (0.6)

Ce qui manque le plus pour les fiches et affiches, indépendamment de la classe.

| Fonction | Pourquoi | Taille |
|---|---|---|
| **Tableaux** (grille de cases avec texte ou image) | Emplois du temps, tableaux de présence, cases du cahier de réussites, grilles de loto, fiches de tri | L |
| **Lignes d'écriture** dans un cadre (Seyès, lignes simples, interlignes adaptés GS) et non seulement en fond de page | Modèles d'écriture du prénom, fiches mixtes image + écriture | M |
| **Texte en pointillés / à repasser** (police ou contour) | Écriture du prénom, graphisme | S |
| **Duplication en grille** : « répéter ce bloc N fois sur la page » | Planches d'étiquettes, cartes de jeux, sans modèle dédié | M |
| **Repères de découpe** et fonds perdus à l'impression | Étiquettes et cartes à découper proprement | S |
| **Formats** A3, A5, et impression d'une affiche A3 sur deux A4 (tuilage) | Affichages, cahiers petit format | M |
| **Pictogrammes de la classe** : catégories (rituels, activités, matériel, émotions) en plus d'OpenMoji, et import d'une banque de pictos (ex. ARASAAC, licence CC BY-NC-SA, à vérifier) | Emploi du temps visuel, ateliers | M |
| **Styles de texte nommés** (titre, consigne, prénom) | Cohérence d'un cahier entier, changement global d'écriture | M |
| Recto-verso et pages en vis-à-vis | Cahiers, livrets | M |
| Rotation libre des blocs, miroir des photos | Mise en page plus libre | S |

### Étape B : la liste de la classe (0.7)

Reprend la tâche déjà prévue « Liste de la classe et pages par enfant » (T-28) et l'élargit.

| Fonction | Détail | Taille |
|---|---|---|
| **Fiche de classe** | Nom, année scolaire, niveaux (PS/MS/GS, classes multi-niveaux), enseignant(s) | S |
| **Élèves** | Prénom, nom facultatif, niveau, date de naissance facultative (anniversaires), photo portrait, groupe(s), couleur | M |
| **Import de la liste** | Coller une colonne de prénoms, ou CSV / tableur exporté de l'ONDE ou d'une feuille de calcul | S |
| **Autorisation photo par élève** | Case « droit à l'image accordé » ; un élève sans autorisation n'apparaît jamais sur une photo de groupe proposée automatiquement, et son portrait est remplacé par un picto choisi | M |
| **Champs de fusion** dans les textes : {prénom}, {PRÉNOM}, {prénom en cursive}, {photo}, {groupe}, {âge} | Base de toute la personnalisation | M |
| **Publipostage** : un document modèle → une page (ou un document) par élève, ou une planche avec tous les élèves | Étiquettes, pages de cahier de vie personnalisées, mots aux parents | M |
| **Générateurs prêts à l'emploi** : étiquettes prénom (3 écritures, avec ou sans photo, formats porte-manteau / casier / table), memory et loto des prénoms, lettres mobiles du prénom, modèle d'écriture du prénom, affiche des anniversaires par mois, tableau des présences, pyramide des âges | Ce que les enseignants refont chaque rentrée | L |

### Étape C : photothèque de la classe (0.8)

| Fonction | Détail | Taille |
|---|---|---|
| **Photothèque de classe** commune à tous les documents de l'année (aujourd'hui chaque document a la sienne) | Dossier local, vignettes, dates | M |
| **Étiquetage des photos par élève** à la main (cliquer sur les enfants présents) | Retrouver « toutes les photos de Léa » ; jamais de reconnaissance faciale envoyée en ligne | M |
| **Étiquetage par activité / sortie / domaine du programme** | Retrouver les photos d'une sortie, d'un domaine pour le carnet de suivi | S |
| **Suggestions** : « Léa n'apparaît que sur 2 photos ce trimestre » | Équité des traces dans les cahiers de vie | S |
| **Tri par lot à l'import** (rafale du téléphone : garder, jeter, étiqueter) | Les enseignants importent des centaines de photos | M |
| Option : regroupement local des visages (modèle embarqué, hors ligne) pour accélérer l'étiquetage | Ambitieux, à évaluer pour la vie privée et le poids du modèle | L |

### Étape D : suivi des apprentissages (0.9 à 1.x)

C'est la fonction la plus attendue et la plus exigeante : à faire en lien avec des conseillers pédagogiques maternelle.

| Fonction | Détail | Taille |
|---|---|---|
| **Référentiel des attendus** du programme de cycle 1 (programme en vigueur, BO du 7 mai 2026 à vérifier), modifiable par l'équipe | Domaines, attendus, formulés « je sais... » | M |
| **Cahier de réussites** : une case par réussite (picto + phrase), marquée réussie à une date, avec photo de la trace | Évaluation positive, seulement les réussites | L |
| **Saisie rapide** : grille élèves × réussites pour cocher pendant ou après l'atelier | Gain de temps réel | M |
| **Génération du carnet de suivi** de chaque élève, période par période, avec mot de l'enseignant | Export PDF / impression par élève ou pour toute la classe | M |
| **Synthèse des acquis de fin de GS** pré-remplie au modèle national | Transmission au CP | M |
| **Continuité sur le cycle** : le carnet suit l'élève de PS à GS, même s'il change d'enseignant (export / import d'un dossier élève) | Équipe de cycle | L |

### Étape E : années scolaires et archives (1.x)

| Fonction | Détail | Taille |
|---|---|---|
| **Espace par année scolaire** : documents, classe, photothèque, suivi | Bascule de rentrée : « nouvelle année » reprend les modèles et les affichages, vide les élèves (ou fait monter les MS en GS) | M |
| **Bibliothèque de documents** : tous les carnets rangés par année, période, type (fiche, affiche, cahier de vie...), avec recherche et vignettes | Fini l'explorateur de fichiers | M |
| **Réutiliser l'an dernier** : ouvrir une fiche de l'an passé et la régénérer avec la nouvelle classe | Gros gain à chaque rentrée | S (avec B) |
| **Archivage et durée de conservation** : à la fin de l'année, proposer d'archiver puis, après un délai réglable, de supprimer photos et données des élèves partis | Respect du RGPD sans effort | M |
| **Sauvegarde** : copie complète vers une clé USB ou un dossier choisi, restauration | Ordinateurs d'école peu fiables | M |
| **Export du dossier d'un élève** (son cahier de vie, son carnet de suivi, ses photos) en PDF pour la famille en fin d'année ou au départ | Remise aux familles | S |

### Étape F : travailler en équipe et au-delà (exploratoire)

| Piste | Remarque |
|---|---|
| **Échange de modèles** entre collègues : paquet de modèles, page de partage sur le site (dépôt de modèles libres sous licence ouverte) | Sans serveur : fichiers, ou dépôt GitHub communautaire |
| **Classe partagée** entre deux enseignants (temps partiel, ATSEM) via un dossier synchronisé (clé, Nextcloud de l'académie) | Gestion des conflits à prévoir |
| **Version tablette** (Android, Tauri mobile) pour photographier et cocher les réussites en classe, synchronisée avec le poste par le réseau local | Ambitieux ; c'est là que sont aujourd'hui les applications concurrentes |
| **Livret pour les familles** : export d'un cahier de vie numérique (PDF ou petite page HTML hors ligne) | Sans hébergement par Cahutin |
| **Assistant de mise en page** (MCP, déjà au plan) étendu : « fais les étiquettes porte-manteau de ma classe », « range les photos de la sortie en 3 pages » | S'appuie sur B et C |
| **Accessibilité et inclusion** : pictos de communication, police adaptée par élève (OpenDyslexic déjà fournie), consignes lues | Élèves à besoins particuliers |
| **Élémentaire** : cahier de réussites cycle 2, plans de travail, ceintures de compétences | Même socle (classe, suivi), autre référentiel |
| **Traduction** de l'interface (les textes de l'interface sont déjà regroupés dans `ui/src/lib/i18n/fr.ts`) pour la Belgique, la Suisse, le Québec | Référentiels différents |

## 4. Ordre proposé et dépendances

```
A (éditeur) ──► B (liste de classe, publipostage) ──► C (photothèque de classe) ──► D (suivi)
                         │                                     │
                         └──────────────► E (années, archives) ◄┘
                                                   │
                                                   ▼
                                           F (équipe, tablette)
```

- **A et B en premier** : bénéfice immédiat à la rentrée (étiquettes, affichages), et B est le socle de tout le reste.
- **Tableaux (A)** avant **cahier de réussites (D)** : les cases de réussite en dépendent.
- **E** peut commencer dès B : la notion d'année scolaire structure la liste de classe ; mieux vaut la poser tôt que la greffer.
- **D** demande le plus de validation pédagogique : à préparer pendant A à C (contacts avec des CPC maternelle, essais avec 2 ou 3 classes).

## 5. Décisions à prendre avant l'étape B

1. **Où vivent les données de classe ?** Un dossier « Cahutin » choisi par l'enseignant (facile à sauvegarder et à synchroniser) ou le dossier de l'application (caché).
2. **Format** : un fichier de classe à part (`.cahutin-classe` ?) ou une base locale (SQLite) ? Le fichier se partage mieux, la base se requête mieux pour la photothèque et le suivi.
3. **Chiffrement** des données élèves sur le disque (poste partagé en salle des maîtres) : avec ou sans mot de passe ?
4. **Ce que contient un `.carnet`** généré pour un élève : la photo de l'enfant y est copiée (comme aujourd'hui) ou référencée ?
5. **Jusqu'où aller vers le suivi** (D), qui rapproche Cahutin des applications d'évaluation et de leurs obligations (déclaration au registre de traitement de l'école, avis du DPD académique).

## Sources

- Cahier de réussites : [exemple Villeurbanne](https://villeurbanne1.circo.ac-lyon.fr/spip/IMG/pdf/cahier_de_reussite_exemple.pdf), [présentation Meurthe-et-Moselle](https://sites.ac-nancy-metz.fr/dsden54-gtd/sites/maternelle54/IMG/pdf/Presentation_cahier_reussite.pdf), [explications Lille](https://conservatoire.etab.ac-lille.fr/files/2022/02/explications-cahier-de-reussite.pdf), [ébauche Granville](https://granville.circonscription.ac-normandie.fr/IMG/pdf/cahierdereussites-ebauche3.pdf)
- Cahier de vie : [présentation Meurthe-et-Moselle](https://sites.ac-nancy-metz.fr/dsden54-gtd/sites/maternelle54/IMG/pdf/Presentation_cahier_vie.pdf), [mémoire DUMAS](https://dumas.ccsd.cnrs.fr/dumas-01358894v1)
- Suivi et synthèse : [Eduscol, suivi et évaluation en maternelle](https://eduscol.education.gouv.fr/4671/suivi-et-evaluation-des-apprentissages-des-eleves-l-ecole-maternelle), [arrêté du 31/12/2015 (Légifrance)](https://www.legifrance.gouv.fr/loda/id/JORFTEXT000031742283), [Moselle, carnet de suivi](https://sites.ac-nancy-metz.fr/maternelle57/2026/03/05/carnet-de-suivi-des-eleves/), [Moselle, synthèse des acquis](https://sites.ac-nancy-metz.fr/maternelle57/2026/05/06/la-synthese-des-acquis-des-eleves-en-fin-decole-maternelle/), [critères d'analyse des carnets (Ain)](https://collectif-maternelle.edu01.ac-lyon.fr/wp-content/uploads/sites/30/2025/05/Criteres-d-analyse-des-carnets-de-suivi-des-apprentissages-1.pdf)
- Affichages et rituels : [rituels et apprentissages (Eure)](https://maternelle27.circonscription.ac-normandie.fr/IMG/pdf/rituels_et_apprentissages_en_maternelle.pdf), [progression des rituels](https://imagesetlangages.fr/PE2/docs_application/maternelle/progression_%20rituels_clermontF.PDF), [aménagement de rentrée (Versailles)](https://ac-versailles.fr/media/22886/download)
- Jeux et prénoms : [propositions d'activités PS (Caen)](https://lepetitprince.etab.ac-caen.fr/IMG/pdf/ps_propositions_activites_1.pdf), [lettres en pagaille](https://darnetal.circonscription.ac-normandie.fr/IMG/pdf/fiche_lettres_en_pagaille-3.pdf), [jeux de société à fabriquer (Lille)](https://pedagogie-nord.ac-lille.fr/docuweb/continuite-pedagogique/docs/jeux-de-societe-a-fabriquer.pdf)
- Outils existants : [Klassroom (Rouen)](https://prim76.ac-normandie.fr/IMG/pdf/klassroom-2.pdf), [carnet de suivi numérique (Strasbourg)](https://circ-ien-riedisheim.site.ac-strasbourg.fr/spip/IMG/pdf/cahier_suivi_num_mat_aw.pdf), [carnet de suivi numérique (Ain)](https://numerique-educatif.edu01.ac-lyon.fr/wp-content/uploads/sites/61/2021/03/carnet_de_suivi_numerique.pdf), [Ma Classe de Maternelle](https://apps.apple.com/us/app/id1539154329), [Kidoo et Kaligo](https://outilstice.com/kidoo-kaligo-applications-mobiles-pour-enseignants-maternelle/)
- Données et image : [RGPD à l'école](https://donneespersonnelles.fr/rgpd-ecoles-education), [captation de l'image et de la voix (Nancy-Metz)](https://sites.ac-nancy-metz.fr/eps/Parcours_de_formation/EPS_aspects_juridiques/docs/dossier_captation.php), [photographie scolaire (Besançon)](https://morteau.circo25.ac-besancon.fr/wp-content/uploads/sites/12/2025/09/Photographie-scolaire.pdf)
