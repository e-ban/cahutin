# Images : photothèque par dossiers suivis, jusqu'à 10 000 photos (2026-11-13)

Mesure de l'étape 4 (dossiers suivis, plus aucune copie de photo), mêmes machine, limites (2 cœurs, 4 Go) et
photos générées que la référence `docs/mesures/images-2026-11-12-reference.md`. Application en version de
développement. Deux passages Timonier, lancés sur la branche `worktree-photo-dossiers` après le commit
`94816f4` (le daemon de Timonier ne répondait pas : les passages ne sont pas enregistrés, les résultats viennent
du JUnit et du journal du banc).

- **Passage complet** : `timonier test --junit <sortie>/mesures.xml -- scripts/mesures/mesurer.sh <sortie> phototheque-suivre phototheque-ouvrir phototheque-enfant`
  (2 h 00 en tout, 12 mesures).
- **Passage du premier parcours seul** (`phototheque-suivre`, 46 min) : refait après coup, parce que le premier
  passage ne notait pas la première grille (voir plus bas). Il attend la grille jusqu'à 20 minutes.
- **Dossiers suivis** : `scripts/mesures/dossiers.sh` fabrique, hors du dépôt, un dossier de n photos pour
  n = 1 000, 5 000 et 10 000 (les n premières photos générées, 50 par sous-dossier). Le parcours ne suit pas les
  sous-dossiers qui sont des liens : ce sont de vraies copies faites par clonage (`cp -a --reflink=always`,
  btrfs), sans espace disque en plus ni copie de 10 000 photos. Chaque taille a sa propre classe (index et
  vignettes à elle).
- **Seuils** : moins de 1 000 ms pour afficher une photothèque déjà parcourue (index et vignettes présents) ;
  pour le premier parcours, seulement un seuil sur le gel de l'interface (300 ms, « quelques centaines »).

## Critère « vignettes présentes »

Le cache de vignettes (`.vignettes/` de la classe) se remplit sur un seul fil en arrière-plan après le parcours.
Le banc compte les fichiers du dossier : la mesure « ouvrir » ne commence que lorsque le cache contient **n
vignettes terminées (`.jpg`), aucune en cours d'écriture (`.tmp`), et que ce compte n'a pas bougé depuis 5
secondes**. La durée du remplissage est notée à part (`phototheque-vignettes`, comptée depuis la fin du premier
parcours).

## Résultats (passage complet)

| Scène | Photos | Temps (ms) | Plus long gel (ms) | Mémoire de pointe cumulée (Mo) | Résultat |
|---|---|---|---|---|---|
| phototheque-suivre | 1000 | 3409 | 35 | 1163 | ok |
| phototheque-vignettes | 1000 | 366688 | | 1358 | ok |
| phototheque-ouvrir | 1000 | 126 | 109 | 1358 | ok |
| phototheque-enfant | 1000 | 18 | 17 | 1358 | ok |
| phototheque-suivre | 5000 | 41590 | 184 | 4096 | ok |
| phototheque-vignettes | 5000 | 1943619 | | 4096 | ok |
| phototheque-ouvrir | 5000 | 518 | 484 | 4096 | ok |
| phototheque-enfant | 5000 | 171 | 170 | 4096 | ok |
| phototheque-suivre | 10000 | 101522 | 410 | 4096 | gel de 410 ms > 300 ms |
| phototheque-vignettes | 10000 | 4718966 | | 4096 | ok |
| phototheque-ouvrir | 10000 | 1122 | 1087 | 4096 | 1122 ms > 1000 ms |
| phototheque-enfant | 10000 | 233 | 228 | 4096 | ok |

« Temps » de `phototheque-suivre` : du clic sur « suivre » à la fin du parcours (la barre d'avancement disparaît).
Les photos arrivent au fil de l'eau pendant ce temps. « Temps » de `phototheque-vignettes` : remplissage complet du
cache par le fil de fond (366 s pour 1 000 photos, 1 944 s soit 32 min pour 5 000, 4 719 s soit 79 min pour
10 000 : environ 2,1 photos par seconde, soit 0,47 s par photo de 4000 x 3000 px sur 2 cœurs).

## Première grille du premier parcours (passage du premier parcours seul)

| Photos | Fin du parcours (ms) | Première grille : 8 vignettes visibles chargées (ms) | Plus long gel (ms) | Résultat |
|---|---|---|---|---|
| 1000 | 3083 | 339915 | 35 | ok |
| 5000 | 37778 | pas venue en 20 min | 240 | délai dépassé (4997 images dans la page, 2563 chargées, 0 cassée) |
| 10000 | 101299 | pas venue en 20 min | 502 | délai dépassé (9997 images, 2034 chargées, 0 cassée) ; gel de 502 ms > 300 ms |

## Lecture

- **Photothèque déjà parcourue** : 126 ms pour 1 000 photos, 518 ms pour 5 000, **1 122 ms pour 10 000**, soit
  un peu au-dessus du seuil de 1 000 ms (corrigé depuis, voir « Après les correctifs » : 325 ms). Presque tout ce temps est du gel du fil de l'interface (1 087 ms) : la
  page de 10 000 vignettes se construit d'un bloc. Filtrer sur un enfant reste rapide (233 ms à 10 000).
- **Premier parcours** : les photos arrivent vite (3 s pour 1 000, 38 à 42 s pour 5 000, 101 s pour 10 000) sans
  bloquer l'interface au-delà de 184 ms à 5 000 photos. À 10 000 photos le plus long gel est de 410 ms (502 ms en
  comptant le temps d'attente de la grille), au-dessus de notre seuil de 300 ms.
- **Problème constaté : les vignettes de la première grille arrivent très tard.** Pendant le premier parcours,
  la page garde une image pour **chaque** photo reçue (4 997 images dans la page pour 5 000 photos, malgré
  `loading="lazy"`) et les vignettes se chargent au rythme du fil de fond (environ 2 par seconde). Hypothèse, non
  vérifiée dans le code : le fil de travail du protocole sert les demandes une par une, dans l'ordre où elles
  arrivent, et fait chaque vignette à la demande ; les 8 vignettes du haut de la grille attendraient derrière
  celles des autres photos (vérifiée depuis, voir « Après les correctifs »).
  Mesuré : 340 s pour 1 000 photos (presque tout le remplissage du cache, 367 s) ; pour 5 000 et 10 000, la
  première grille n'est toujours pas là après 20 minutes (2 563 et 2 034 vignettes chargées). Une grille de
  photos reste donc vide pendant plusieurs minutes après le premier suivi d'un gros dossier, alors que la liste
  est déjà complète. Aucune image n'est cassée : tout finit par arriver. Corrigé depuis (voir « Après les
  correctifs ») : 7,7 s à 1 000 photos, 39 s à 10 000.
- **Mémoire** : la pointe cumulée (application et processus web, un seul cgroup limité à 4 Go) monte à 1,2 à
  1,4 Go avec 1 000 photos, puis **atteint la limite de 4 096 Mo** dès les 5 000 photos. Le cgroup compte aussi le
  cache de fichiers du noyau (le fil de fond relit les originaux de 3 à 5 Mo, plus de 40 Go pour 10 000 photos),
  qui se libère tout seul : ce chiffre ne dit donc pas à lui seul que l'application « consomme » 4 Go. Aucune
  application tuée pendant les 2 heures de mesure. Une mesure de la mémoire propre à l'application (hors cache
  de fichiers) reste à faire.
- **Remplissage du cache** : plus d'une heure et quart pour 10 000 photos sur 2 cœurs et un seul fil, avec la
  page ouverte. Il reprend où il s'était arrêté à chaque ouverture (les vignettes faites sont gardées).

## Limites de la mesure

- Photos générées de 4000 x 3000 px, 3 à 5,5 Mo : plus lourdes que beaucoup de photos de classe, donc des
  vignettes plus lentes à faire que chez un enseignant ; l'ordinateur de mesure a un disque NVMe.
- « Ouvrir » rouvre la page à froid (liste vidée, index relu depuis le disque) ; le parcours de rafraîchissement
  part en même temps, comme chez l'enseignant.
- Une seule mesure par scène et par taille : la variation d'un passage à l'autre n'est pas connue (le premier
  parcours mesuré deux fois donne 3 409 et 3 083 ms à 1 000 photos, 41 590 et 37 778 ms à 5 000, 101 522 et
  101 299 ms à 10 000).

## Après les correctifs (2026-11-13)

Mêmes machine, limites, photos et dossiers, application en version de développement, sans Timonier. Passage
retenu sur la branche `worktree-photo-dossiers` au commit `3dcb0d5` :
`PHOTOS=… DOSSIERS=… TAILLES=1000,10000 scripts/mesures/mesurer.sh <sortie> phototheque-suivre phototheque-ouvrir`
(27 min). Un premier passage identique, fait avant le dernier correctif de la grille (commit `460afa6`, avec
`TAILLES` pas encore enregistré), est donné entre parenthèses. Une seule mesure par cas.

### Causes, vérifiées

- **Ouverture lente à 10 000 photos** : la page dessinait les 10 000 vignettes (bouton, image, pastilles) d'un
  coup, d'où un gel qui croît avec le nombre de photos.
- **Première grille tardive** (passage instrumenté à 1 000 photos, journal temporaire de nombres, retiré
  ensuite) : à la fin du parcours (3 s), la page avait déjà demandé 947 vignettes, 997 en tout. Chaque paquet de
  50 photos reçu est plus récent que les précédents et arrive donc en haut de la grille, à l'écran ; ses images
  sont demandées avant d'être poussées vers le bas par le paquet suivant (déduit de l'ordre des paquets et du
  tri de la grille : le journal montre les demandes, pas leur raison). Le fil du protocole les servait dans
  l'ordre d'arrivée (les 81 premières réponses : les photos 0 à 84 de l'index, attente de 40 s pour la 80e) ;
  les vignettes visibles, demandées en dernier, attendaient derrière toutes les autres. Chacune prenait 500 ms,
  dont 450 ms de réduction Lanczos3 sur l'image de 12 Mpx. Le fil de fond, parti après le parcours dans l'ordre
  de l'index, refaisait en même temps les mêmes photos que le protocole. Pas de contention de verrou : le fil de
  fond travaille sur une copie de l'index et la vignette se fait sans verrou.

### Correctifs

- La grille dessine 300 vignettes, puis 300 de plus quand son bas approche de l'écran (regardé à chaque
  défilement, y compris après un saut tout en bas) ; un jour loin de l'écran n'est ni mis en page ni dessiné
  (`content-visibility`). Les jours, filtres, le choix et la vue en grand portent toujours sur toutes les photos
  montrées ; un autre filtre repart de 300.
- Le fil du protocole sert la demande la plus récente d'abord (toujours un seul fil).
- La vignette du cache se fait par une réduction par moyenne jusqu'à 512 px, puis Lanczos3 : 109 ms au lieu de
  500 ms par photo de 4000 x 3000 px, image presque identique (écart moyen 0,15 sur 255).

### Résultats

| Scène | Photos | Avant (ms) | Après (ms) | Plus long gel avant / après (ms) |
|---|---|---|---|---|
| phototheque-suivre, fin du parcours | 1000 | 3409 | 5868 (3077) | 35 / 31 (28) |
| phototheque-suivre, première grille | 1000 | 339915 | **7727** (4831) | |
| phototheque-vignettes | 1000 | 366688 | 50251 (50236) | |
| phototheque-ouvrir | 1000 | 126 | 81 (56) | 109 / 47 (40) |
| phototheque-suivre, fin du parcours | 10000 | 101522 | 64569 (75962) | 410 / **72** (62) |
| phototheque-suivre, première grille | 10000 | pas venue en 20 min | **39329** (45195) | |
| phototheque-vignettes | 10000 | 4718966 | 777861 (768946) | |
| phototheque-ouvrir | 10000 | 1122 | **325** (347) | 1087 / 291 (313) |

Défilement jusqu'à la dernière des 10 000 photos (essai à part, sauts répétés tout en bas de la page, après un
premier parcours) : 2,4 s et 68 ms de gel au plus ; sans `content-visibility`, 25,6 s et 813 ms de gel quand
près de 10 000 vignettes sont dessinées. Après un changement de filtre, la grille repart bien de 300 vignettes.

- **Ouvrir une photothèque de 10 000 photos** : 325 ms, sous le seuil de 1 000 ms. Il reste 291 ms de gel : le
  tri, le regroupement par jour et les comptes (par enfant, par activité, par dossier) se font encore sur les
  10 000 photos.
- **Première grille à 1 000 photos** : 1,9 s après la fin du parcours (1,8 s au premier passage). Les photos
  les plus récentes arrivent en dernier, la grille ne se stabilise qu'à la fin du parcours. La fin du parcours
  varie d'un passage à l'autre (3,1 et 5,9 s).
- **Première grille à 10 000 photos** : 39 s, alors que le parcours dure 65 s. Les sous-dossiers sont parcourus
  par ordre alphabétique (`sortie-01` … `sortie-09`, `sortie-10`, `sortie-100` … `sortie-109`, `sortie-11`, …) :
  les photos les plus récentes (`sortie-200`, mars) arrivent en 121e position sur 200. Mesuré dans trois essais :
  elles arrivent dans la page 2,0 s avant la première grille (43,2 s puis 45,2 s ; 40,2 puis 42,2 ; 37,5 puis
  39,5). Avant cela, chaque nouveau paquet plus récent remplace le haut de la grille deux à trois fois par
  seconde, et aucune vignette n'a le temps d'arriver. **Un dossier rangé par date ne montre donc ses vignettes
  qu'à la fin du parcours** (environ 65 s à 10 000 photos). Pour faire mieux, il faudrait changer l'ordre dans
  lequel le parcours annonce les photos (les plus récentes d'abord) ou garder le haut de la grille immobile
  pendant le parcours.
- **Remplissage du cache** : 50 s pour 1 000 photos (367 s avant), 13 min pour 10 000 (79 min avant).
- **Gel du premier parcours à 10 000 photos** : 72 ms (410 ms avant) : la page ne redessine plus que 300
  vignettes à chaque paquet.
- **Mémoire** : la pointe cumulée du cgroup atteint 4 096 Mo (la limite) dès 1 000 photos dans le passage
  retenu (2 423 Mo au premier passage, 1 163 avant) ; elle compte le cache de fichiers du noyau, et les originaux
  sont relus plus vite. Aucune application tuée. La mémoire propre à l'application reste à mesurer.
- Non remesuré : 5 000 photos, le filtre d'un enfant (`phototheque-enfant`).
