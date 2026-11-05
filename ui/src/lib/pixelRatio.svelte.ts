// Mise à l'échelle de l'écran où se trouve la fenêtre (`devicePixelRatio`), suivie quand la fenêtre change
// d'écran ou que l'échelle change : une requête média sur la valeur courante prévient de son changement.
class PixelRatio {
  value = $state(typeof window === "undefined" ? 1 : window.devicePixelRatio || 1);

  constructor() {
    if (typeof window === "undefined" || !window.matchMedia) return;
    const watch = () => {
      const q = window.matchMedia(`(resolution: ${window.devicePixelRatio}dppx)`);
      q.addEventListener(
        "change",
        () => {
          this.value = window.devicePixelRatio || 1;
          watch();
        },
        { once: true },
      );
    };
    watch();
  }
}

export const pixelRatio = new PixelRatio();
