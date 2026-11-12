# Images : mesures de référence avant refonte (2026-11-12)

Référence de l'étape 1 de la refonte de la gestion des images
(`docs/superpowers/specs/2026-11-12-gestion-des-images-design.md`), avant les étapes 2 à 4.

- **Commit mesuré** : `601840f` (application en version de développement, bibliothèques d'images optimisées).
- **Machine** : AMD Ryzen 7 9800X3D, 30 Go, disque NVMe, btrfs, Linux 7.3 (CachyOS).
- **Limites imposées à l'application** : 2 cœurs (`taskset -c 0,1`), 4 Go de mémoire sans échange
  (`systemd-run -p MemoryMax=4G`). Le disque dur classique **n'est pas simulé** : les lectures de photos sont
  ici bien plus rapides que sur un ordinateur d'école.
- **Photos** : générées par `scripts/mesures/photos.sh`, 4000 x 3000 px, 3,1 à 5,5 Mo, une sur vingt avec date
  EXIF.
- **Lancement** : `timonier test --junit <sortie>/mesures.xml -- scripts/mesures/mesurer.sh <sortie>`
  (passage Timonier R-8).
- **Seuils** : moins de 1 000 ms jusqu'au premier écran utile ; aucun gel de l'interface au-delà de 100 ms
  pendant un import ou un défilement.

| Scène | Photos | Temps (ms) | Plus long gel (ms) | Mémoire de pointe cumulée (Mo) | Résultat |
|---|---|---|---|---|---|
| tiroir-import | 100 | 71584 | 51 | 1623 | ok |
| tiroir-ouvrir | 100 | 22 | 18 | 1623 | ok |
| tiroir-filtre | 100 | 54 | 28 | 1623 | ok |
| tiroir-import | 300 | 143187 | 28 | 1811 | ok |
| tiroir-ouvrir | 300 | 51 | 43 | 1811 | ok |
| tiroir-filtre | 300 | 65 | 32 | 1811 | ok |
| tiroir-import | 500 | 143530 | 40 | 2117 | ok |
| tiroir-ouvrir | 500 | 78 | 66 | 2117 | ok |
| tiroir-filtre | 500 | 57 | 31 | 2117 | ok |
| tiroir-defiler | 500 | 826 | 18 | 2117 | ok |
| phototheque-lot | 1000 | 36834 | 152 | 3052 | gel de 152 ms > 100 ms |
| phototheque-ouvrir | 1000 | 85 | 85 | 3052 | ok |
| phototheque-enfant | 1000 | 18 | 17 | 3052 | ok |

Comment lire les scènes :

- `tiroir-import` : tailles cumulées (100, puis 200 de plus, puis 200 de plus) ; le temps est celui du lot.
- `tiroir-filtre` : clic sur le dossier de la dernière sortie (50 photos en bas de la liste, pas encore
  chargées) ; seules les vignettes que le filtre fait charger comptent. « Non utilisées » n'est pas mesuré :
  sans photo posée, il ne change rien.
- `tiroir-defiler` : le temps est celui du défilement jusqu'en bas ; seul le gel a un seuil.
- `phototheque-lot` : une sortie de 50 photos ajoutée à une photothèque qui en compte déjà 950 ; on n'ajoute
  jamais des milliers de photos d'un coup.
- `phototheque-enfant` : prêt quand la grille ne montre plus que les photos de l'enfant. Ses vignettes étaient
  déjà chargées : la mesure dit que le filtre s'affiche en une image (18 ms), pas le temps de chargement.
- Mémoire : l'application et ses processus web (qui dessinent la page et décodent les vignettes), pointe depuis
  le lancement, d'où une colonne qui ne fait que monter.

## Lecture

- **Au-delà du seuil** : le gel de l'interface pendant un import. Il varie beaucoup d'un passage à l'autre
  pour le même lot : 446, 205, 107, 34, 33 et 51 ms pour 100 photos dans le tiroir ; 57, 118, 40 et 152 ms
  pour une sortie dans la photothèque. L'import gèle donc l'interface par à-coups ; il faudra plusieurs
  passages pour juger une amélioration.
- **Le plus lent** : l'import lui-même, environ 0,7 s par photo sur 2 cœurs, dans le tiroir comme dans la
  photothèque. Hors seuil (il ne bloque pas l'écran), mais c'est l'attente que l'enseignant ressent.
- **La mémoire** : 3 Go pour une photothèque de 1 000 photos, sous la limite de 4 Go mais tout près. C'est le
  risque le plus net pour les grandes photothèques.
- **Pour les étapes 2 à 4** : l'affichage tient déjà le seuil à cette échelle (ouvrir, filtrer : moins de
  100 ms ; défiler sans gel), donc l'étape 2 peut changer la présentation sans craindre la vitesse. Les enjeux
  sont le gel pendant l'import et la mémoire. Les photothèques de 5 000 et 10 000 photos seront mesurées à
  l'étape 4, quand les dossiers suivis éviteront les copies (aujourd'hui, en remplir une de 10 000 prendrait
  deux heures).
