# cahutin-mcp (essai)

Serveur MCP qui laisse un assistant (LLM) lire et modifier la **mise en page** d'un document Cahutin, sans jamais lui montrer ni texte, ni photo, ni nom de fichier.

## Ce que voit l'assistant

Pour chaque page : sa taille et son fond. Pour chaque élément : son type (texte, photo, forme, image OpenMoji, tableau), son cadre en millimètres, sa rotation, son groupe, s'il est verrouillé, et son style (police, taille, couleur, bordure). Un texte dit seulement s'il est vide (et s'il a des lignes d'écriture) ; une photo, si sa case est remplie, ses proportions et si elle est en miroir ; un tableau, ses lignes, ses colonnes et la place de chaque case (les cases sont des textes, vus comme les autres).

La vue est construite champ par champ (`src/view.rs`) : un champ ajouté au modèle n'est pas envoyé tant qu'il n'y est pas écrit. Des tests vérifient qu'aucun texte, invite, titre, identifiant de photo ni chemin ne sort, sur un document piégé, un tableau et tous les exemples.

## Outils

- `get_layout` : la mise en page (tout le document ou une page) ;
- `set_frame` : déplacer, redimensionner, tourner un élément ;
- `set_text_style` : police, taille, couleur, alignement d'un texte ;
- `set_background` : couleur et motif d'une page.

Chaque modification réécrit seulement `document.json` dans le fichier ; photos et polices sont recopiées sans être décompressées. Les éléments verrouillés sont refusés. Un tableau déplacé emporte ses cases ; une case seule ne se déplace pas.

## Essayer

```sh
cargo build --release -p cahutin-mcp
claude mcp add cahutin -- "$PWD/target/release/cahutin-mcp" /chemin/vers/document.carnet
```

Fermer le document dans Cahutin pendant l'essai, puis le rouvrir pour voir les changements.

Le serveur lui-même ne se connecte à rien ; l'assistant, lui, envoie à son fournisseur ce que renvoient les outils, c'est-à-dire la mise en page seule.
