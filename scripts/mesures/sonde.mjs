// La sonde injectée dans la page de l'application : le temps d'une action jusqu'à l'état « prêt », et le plus
// long gel de l'interface pendant ce temps (le plus grand écart entre deux images d'animation).
export const SONDE = `
window.__mesure = (action, pret, delaiMs = 120000) =>
  new Promise((resolve) => {
    const debut = performance.now();
    let derniere = debut;
    let gelMaxMs = 0;
    let fini = false;
    const finir = (expire, erreur) => {
      if (fini) return;
      fini = true;
      const r = { ms: Math.round(performance.now() - debut), gelMaxMs: Math.round(gelMaxMs), expire };
      resolve(erreur ? { ...r, erreur } : r);
    };
    const image = (t) => {
      if (fini) return;
      gelMaxMs = Math.max(gelMaxMs, t - derniere);
      derniere = t;
      let ok = false;
      try { ok = !!pret(); } catch { ok = false; }
      if (ok) finir(false);
      else if (performance.now() - debut >= delaiMs) finir(true);
      else requestAnimationFrame(image);
    };
    // L'action peut être longue (un import) : l'état prêt est guetté pendant qu'elle tourne. Si elle échoue,
    // la mesure s'arrête tout de suite avec son erreur.
    Promise.resolve()
      .then(action)
      .catch((e) => finir(false, String(e)));
    requestAnimationFrame(image);
  });

const chargee = (img) => img.complete && img.naturalWidth > 0;

/** Marque les vignettes déjà chargées : un filtre ne compte que celles qu'il fait charger. */
window.__marquerChargees = (selecteur) => {
  for (const img of document.querySelectorAll(selecteur + " img")) {
    if (chargee(img)) img.dataset.mesureAvant = "1";
    else delete img.dataset.mesureAvant;
  }
};

/** Vrai quand au moins min(8, attendues) vignettes visibles, chargées et non marquées sont là. */
window.__vignettesPretes = (selecteur, attendues) => {
  const voulu = Math.min(8, attendues);
  if (voulu === 0) return true;
  const h = window.innerHeight, w = window.innerWidth;
  let n = 0;
  for (const img of document.querySelectorAll(selecteur + " img")) {
    if (img.dataset.mesureAvant) continue;
    const r = img.getBoundingClientRect();
    const visible = r.width > 0 && r.bottom > 0 && r.top < h && r.right > 0 && r.left < w;
    if (visible && chargee(img)) n++;
    if (n >= voulu) return true;
  }
  return false;
};
`;
