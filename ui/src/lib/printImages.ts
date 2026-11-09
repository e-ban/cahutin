/**
 * Attend que chaque image soit chargée et décodée : les pages d'impression montrent les photos entières, que
 * l'écran n'a souvent chargées qu'en vignette. Une image en erreur n'empêche pas d'imprimer le reste.
 */
export async function imagesDecoded(images: Iterable<{ decode(): Promise<unknown> }>): Promise<void> {
  await Promise.all([...images].map((i) => i.decode().catch(() => {})));
}
