// Téléchargement direct depuis les releases GitHub : le système est deviné, la version et le format se
// choisissent. Sans réponse de GitHub, le bouton mène à la page de la dernière version.
(function () {
  const DEPOT = "e-ban/cahutin";
  const FORMATS = { exe: /x64-setup\.exe$/, AppImage: /\.AppImage$/, deb: /\.deb$/ };
  const bouton = document.getElementById("dl-bouton");
  const format = document.getElementById("dl-format");
  const version = document.getElementById("dl-version");
  let versions = [];

  // Windows par défaut ; Linux si le navigateur le dit.
  if (/Linux/.test(navigator.userAgent) && !/Android/.test(navigator.userAgent)) format.value = "AppImage";

  function maj() {
    const v = versions[version.selectedIndex];
    const fichier = v && v.assets.find((a) => FORMATS[format.value].test(a.name));
    const sys = format.value === "exe" ? "Windows" : "Linux";
    if (fichier) {
      bouton.href = fichier.browser_download_url;
      bouton.textContent = `Télécharger pour ${sys} (${v.tag_name.replace(/^v/, "")})`;
    } else {
      bouton.href = v ? v.html_url : `https://github.com/${DEPOT}/releases/latest`;
      bouton.textContent = `Télécharger pour ${sys}`;
    }
  }

  format.addEventListener("change", maj);
  version.addEventListener("change", maj);
  maj();

  fetch(`https://api.github.com/repos/${DEPOT}/releases?per_page=30`)
    .then((r) => (r.ok ? r.json() : Promise.reject(r.status)))
    .then((liste) => {
      versions = liste.filter((v) => !v.draft && v.assets.length);
      if (!versions.length) return;
      version.replaceChildren(
        ...versions.map((v, i) => {
          const o = document.createElement("option");
          o.textContent = v.tag_name.replace(/^v/, "") + (i === 0 ? " (la plus récente)" : "") + (v.prerelease ? " (préversion)" : "");
          return o;
        }),
      );
      maj();
    })
    .catch(() => {});

  // Barre du haut : la section affichée est surlignée.
  const liens = [...document.querySelectorAll(".barre .liens a")];
  const lienDe = new Map();
  const vue = new IntersectionObserver(
    (entrees) => {
      for (const e of entrees) {
        if (e.isIntersecting) for (const a of liens) a.classList.toggle("ici", a === lienDe.get(e.target));
      }
    },
    { rootMargin: "-45% 0px -50% 0px" },
  );
  for (const a of liens) {
    const section = document.querySelector(a.getAttribute("href"))?.closest("section");
    if (section) {
      lienDe.set(section, a);
      vue.observe(section);
    }
  }
})();
