// Poser un champ depuis le tiroir Champs ou le bouton « Champ » : un champ d'élève devient une pastille ; un champ
// de la date ou de la classe s'écrit aussitôt (calendrier pour une date), sauf en mode modèle.
import { fillAutoFields } from "./api";
import { addsAutoFields, completeText, fillClassFields, placing } from "./autoFields";
import { classes } from "./classStore.svelte";
import { dateAsk } from "./dateAsk.svelte";
import { editor } from "./docStore.svelte";
import { addField, appendText, fieldText, textWith } from "./fieldEdit";
import { fr } from "./i18n/fr";
import { insertField, insertText } from "./textFormat.svelte";
import type { Class, RichText } from "./types";

/** `target` : le texte choisi (barre), ou `null` (tiroir : au curseur si l'on écrit, sinon un nouveau texte). */
export async function placeField(key: string, target: string | null) {
  const into = editor.editingText ?? target;
  const p = await placing(key, { templateMode: editor.templateMode, cls: classes.current?.class ?? null, ask: (c) => dateAsk.ask(c), fill: fillAutoFields });
  if (p.kind === "cancel") return;
  if (p.kind === "pastille" && p.notice) editor.notice = waitingNotice(p.notice);
  const words = p.kind === "text" ? p.text : null;
  // La fenêtre de date a pu fermer l'écriture : on écrit alors dans le texte lui-même.
  if (into && editor.editingText === into) {
    if (words === null) insertField(key);
    else insertText(words);
  } else if (into) {
    editor.updateElement(into, (e) => (e.kind.type === "Text" ? { ...e, kind: { ...e.kind, content: words === null ? addField(e.kind.content, key) : appendText(e.kind.content, words) } } : e));
  } else {
    const el = words === null ? fieldText(editor.page, key) : textWith(editor.page, words);
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, el] }));
    editor.selection = [el.id];
  }
}

/** Le message pour des champs de la classe qui attendent. */
export const waitingNotice = (w: "noClass" | "empty") => (w === "noClass" ? fr.autoFieldsNeedClass : fr.autoFieldEmpty);

/** Un texte qu'on vient d'écrire, avec un champ de la date ou de la classe tapé au clavier : il s'écrit comme un
 *  champ posé (calendrier pour une date), hors mode modèle. */
export async function completeTypedText(id: string, before: RichText, after: RichText) {
  if (editor.templateMode || !addsAutoFields(before, after)) return;
  const done = await completeText(after, { cls: classes.current?.class ?? null, ask: (c) => dateAsk.ask(c), fill: fillAutoFields });
  if (done.notice) editor.notice = waitingNotice(done.notice);
  if (JSON.stringify(done.content) === JSON.stringify(after)) return;
  editor.updateElement(id, (e) => (e.kind.type === "Text" ? { ...e, kind: { ...e.kind, content: done.content } } : e));
}

/** La classe choisie ou complétée : les champs de la classe qui attendent dans le document s'écrivent. */
export async function fillWaitingClassFields(cls: Class) {
  if (editor.templateMode) return;
  const doc = editor.doc;
  const pages = await fillClassFields(doc.pages, cls, fillAutoFields);
  // Le document a changé pendant le calcul : on ne l'écrase pas.
  if (!pages || editor.doc !== doc) return;
  editor.commit({ ...doc, pages });
  // Le message qui disait que ces champs attendaient n'est plus vrai.
  if (editor.notice === fr.autoFieldsNeedClass || editor.notice === fr.autoFieldEmpty) editor.notice = null;
}
