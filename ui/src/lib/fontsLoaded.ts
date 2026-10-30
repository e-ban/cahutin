// Un texte mesuré avant le chargement de sa police l'a été avec une police de remplacement : on le
// mesure de nouveau quand les polices sont prêtes, puis à chaque police chargée plus tard.

type FontSet = EventTarget & { ready: Promise<unknown> };

/** Appelle `cb` quand les polices sont prêtes et après chaque chargement ; rend la fonction qui arrête. */
export function onFontsLoaded(fonts: FontSet, cb: () => void): () => void {
  let stopped = false;
  const listener = () => {
    if (!stopped) cb();
  };
  fonts.ready.then(listener);
  fonts.addEventListener("loadingdone", listener);
  return () => {
    stopped = true;
    fonts.removeEventListener("loadingdone", listener);
  };
}
