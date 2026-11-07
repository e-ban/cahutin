// Poser un champ depuis le tiroir Texte ou le bouton « Champ » : un champ d'élève devient une pastille ; un champ
// de la date ou de la classe s'écrit aussitôt (calendrier pour une date), sauf en mode modèle.
import { fillAutoFields } from "./api";
import { autoText } from "./autoFields";
import { isAutoField } from "./classFields";
import { classes } from "./classStore.svelte";
import { dateAsk } from "./dateAsk.svelte";
import { editor } from "./docStore.svelte";
import { appendText, fieldText, setField, textWith } from "./fieldEdit";
import { fr } from "./i18n/fr";
import { insertField, insertText } from "./textFormat.svelte";

/** `target` : le texte choisi (barre), ou `null` (tiroir : au curseur si l'on écrit, sinon un nouveau texte). */
export async function placeField(key: string, target: string | null) {
  const into = editor.editingText ?? target;
  let words: string | null = null;
  if (isAutoField(key) && !editor.templateMode) {
    const t = await autoText(key, { cls: classes.current?.class ?? null, ask: (c) => dateAsk.ask(c), fill: fillAutoFields });
    if (t === null) return;
    if (t === `{${key}}`) editor.notice = fr.autoFieldsNeedClass;
    else words = t;
  }
  // La fenêtre de date a pu fermer l'écriture : on écrit alors dans le texte lui-même.
  if (into && editor.editingText === into) {
    if (words === null) insertField(key);
    else insertText(words);
  } else if (into) {
    editor.updateElement(into, (e) => (e.kind.type === "Text" ? { ...e, kind: { ...e.kind, content: words === null ? setField(e.kind.content, key) : appendText(e.kind.content, words) } } : e));
  } else {
    const el = words === null ? fieldText(editor.page, key) : textWith(editor.page, words);
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, el] }));
    editor.selection = [el.id];
  }
}
