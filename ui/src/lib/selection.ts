// Choisir plusieurs images : clic (celle-ci seule), Ctrl (en ajouter ou en retirer une), Maj (toutes celles entre
// deux), comme sur la page Photothèque.

/** La sélection après un clic sur `key`. `order` : les images dans l'ordre affiché ; `last` : la dernière cliquée. */
export function pick(selected: string[], order: string[], key: string, mods: { ctrl: boolean; shift: boolean }, last: string | null): string[] {
  if (mods.shift && last !== null && order.includes(last)) {
    const [a, b] = [order.indexOf(last), order.indexOf(key)].sort((x, y) => x - y);
    return [...new Set([...selected, ...order.slice(a, b + 1)])];
  }
  if (mods.ctrl) return selected.includes(key) ? selected.filter((k) => k !== key) : [...selected, key];
  return selected.length === 1 && selected[0] === key ? [] : [key];
}

/** Tout un groupe : ajouté, ou retiré s'il est déjà choisi en entier. */
export function toggleAll(selected: string[], keys: string[]): string[] {
  return keys.every((k) => selected.includes(k)) ? selected.filter((k) => !keys.includes(k)) : [...new Set([...selected, ...keys])];
}
