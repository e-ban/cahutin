// Préférences du poste (panneau ouvert, taille des vignettes, astuces, documents récents) dans localStorage.
// Le stockage peut être indisponible ou vidé : on retombe alors sur la valeur par défaut.
const PREFIX = "carnet.";

export function readPref<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(PREFIX + key);
    return raw === null ? fallback : (JSON.parse(raw) as T);
  } catch {
    return fallback;
  }
}

export function writePref<T>(key: string, value: T): void {
  try {
    localStorage.setItem(PREFIX + key, JSON.stringify(value));
  } catch {
    // Stockage indisponible : la préférence ne survivra pas à la session.
  }
}
