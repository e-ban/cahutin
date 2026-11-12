// La sonde injectée dans la page de l'application : le temps d'une action jusqu'à l'état « prêt », et le plus
// long gel de l'interface pendant ce temps (le plus grand écart entre deux images d'animation).
export const SONDE = `
window.__mesure = (action, pret, delaiMs = 120000) =>
  new Promise((resolve) => {
    const debut = performance.now();
    let derniere = debut;
    let gelMaxMs = 0;
    let fini = false;
    const finir = (expire) => {
      if (fini) return;
      fini = true;
      resolve({ ms: Math.round(performance.now() - debut), gelMaxMs: Math.round(gelMaxMs), expire });
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
    // L'action peut être longue (un import) : l'état prêt est guetté pendant qu'elle tourne.
    Promise.resolve()
      .then(action)
      .catch((e) => { console.error("mesure", e); });
    requestAnimationFrame(image);
  });

/** Vrai quand au moins min(8, attendues) vignettes visibles dans la fenêtre sont chargées. */
window.__vignettesPretes = (selecteur, attendues) => {
  const voulu = Math.min(8, attendues);
  if (voulu === 0) return true;
  const h = window.innerHeight, w = window.innerWidth;
  let n = 0;
  for (const img of document.querySelectorAll(selecteur + " img")) {
    const r = img.getBoundingClientRect();
    const visible = r.width > 0 && r.bottom > 0 && r.top < h && r.right > 0 && r.left < w;
    if (visible && img.complete && img.naturalWidth > 0) n++;
    if (n >= voulu) return true;
  }
  return false;
};
`;
