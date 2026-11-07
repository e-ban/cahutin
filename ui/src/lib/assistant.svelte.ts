// Assistant de mise en page : le fil de la conversation et la proposition en cours. Les étapes s'affichent
// sur la page au fur et à mesure (editor.showProposal) ; rien n'entre dans l'historique avant « Accepter ».
import * as api from "./api";
import { editor } from "./docStore.svelte";
import { modal } from "./modal.svelte";
import { templates } from "./templates.svelte";
import type { AssistantDone, AssistantSettings, AssistantStep, Document, Element } from "./types";

export type Outcome = "pending" | "accepted" | "refused" | "none";
export type Entry =
  | { kind: "user"; text: string }
  | { kind: "assistant"; text: string; steps: AssistantStep[]; outcome: Outcome }
  | { kind: "error"; text: string };

export interface Proposal {
  base: Document;
  steps: AssistantStep[];
}

const lastDoc = (p: Proposal) => p.steps[p.steps.length - 1].document;

const words = (e: Element) => (e.kind.type === "Text" ? e.kind.content.paragraphs.map((p) => p.runs.map((r) => r.text).join("")).join("\n") : null);

/** Les textes que l'assistant a écrits, et leurs copies nouvelles (« Répéter », dupliquer) : le modèle les garde.
 *  La copie d'un texte de l'enseignant n'en fait pas partie, comme son original. */
function writtenWithCopies(p: Proposal, final: Document): string[] {
  const written = p.steps.flatMap((s) => s.written);
  const all = final.pages.flatMap((pg) => pg.elements);
  const before = new Set(p.base.pages.flatMap((pg) => pg.elements).map((e) => e.id));
  const theirs = new Set(all.filter((e) => written.includes(e.id)).map(words).filter((w) => !!w?.trim()));
  const copies = all.filter((e) => !before.has(e.id) && !written.includes(e.id) && theirs.has(words(e)));
  return [...written, ...copies.map((e) => e.id)];
}

/** La page affichée suit la modification : celle de l'élément touché, ou la page ajoutée. */
function followPage(step: AssistantStep, pagesBefore: number) {
  const pages = step.document.pages;
  const at = step.touched.length ? pages.findIndex((p) => p.elements.some((el) => step.touched.includes(el.id))) : -1;
  if (at >= 0) editor.pageIndex = at;
  else if (pages.length > pagesBefore) editor.pageIndex = pages.length - 1;
}

class Assistant {
  settings = $state<AssistantSettings | null>(null);
  open = $state(false);
  busy = $state(false);
  thread = $state.raw<Entry[]>([]);
  proposal = $state.raw<Proposal | null>(null);
  /** Étape affichée (de la proposition ou d'une demande passée) ; null : la fin. */
  viewing = $state.raw<AssistantStep | null>(null);
  /** « Arrêter » cliqué, en attendant que le modèle rende la main. */
  stopping = $state(false);
  #running: Promise<void> | null = null;
  /** Échanges à renvoyer au modèle pour continuer : jusqu'à « Accepter » ou « Refuser », même sans
   *  modification (le modèle a pu poser une question). */
  #history: unknown[] = [];

  get enabled(): boolean {
    return this.settings?.enabled === true;
  }

  async load() {
    try {
      this.settings = await api.assistantSettings();
    } catch {
      this.settings = null;
    }
  }

  async send(text: string) {
    text = text.trim();
    if (!text || this.busy) return;
    this.view(null);
    this.thread = [...this.thread, { kind: "user", text }];
    const p = this.proposal;
    const steps: AssistantStep[] = [];
    let done: AssistantDone | null = null;
    this.busy = true;
    const request = { document: p ? lastDoc(p) : editor.doc, page: editor.pageIndex, selection: editor.selection, history: this.#history, text };
    // Tauri livre les gros messages (chaque étape porte le document) après coup : la demande n'est finie
    // qu'à la réception de « Done », pas au retour de l'appel.
    let finished!: () => void;
    const doneReceived = new Promise<void>((r) => (finished = r));
    this.#running = api
      .assistantSend(request, (e) => {
        if (e.type === "Done") {
          done = e;
          finished();
          return;
        }
        steps.push(e);
        const cur = this.proposal ?? { base: editor.doc, steps: [] };
        this.proposal = { ...cur, steps: [...cur.steps, e] };
        const before = editor.doc.pages.length;
        editor.showProposal(e.document);
        followPage(e, before);
      })
      .then(() => doneReceived)
      .catch((e) => void (this.thread = [...this.thread, { kind: "error", text: String(e) }]));
    await this.#running;
    this.#running = null;
    this.busy = false;
    this.stopping = false;
    const end = done as AssistantDone | null;
    if (!end) return;
    this.#history = end.history;
    const outcome: Outcome = steps.length ? "pending" : "none";
    // Une erreur sans rien de fait : l'erreur seule, pas « Aucune modification » avant elle.
    const entries: Entry[] = end.error && !end.reply && !steps.length ? [] : [{ kind: "assistant", text: end.reply, steps, outcome }];
    if (end.error) entries.push({ kind: "error", text: end.error });
    this.thread = [...this.thread, ...entries];
  }

  stop() {
    if (!this.busy) return;
    this.stopping = true;
    void api.assistantStop().catch(() => {});
  }

  /** L'assistant vient d'être désactivé : rien de lui ne reste sur la page. */
  turnedOff() {
    this.refuse();
    this.view(null);
    this.open = false;
  }

  #close(outcome: Outcome) {
    this.thread = this.thread.map((e) => (e.kind === "assistant" && e.outcome === "pending" ? { ...e, outcome } : e));
    this.proposal = null;
    this.viewing = null;
    this.#history = [];
  }

  async accept() {
    const p = this.proposal;
    if (!p || this.busy) return;
    const final = lastDoc(p);
    const written = writtenWithCopies(p, final);
    for (const s of p.steps) {
      if (!s.template) continue;
      try {
        await api.savePageAsTemplate(s.template.name, final.pages[s.template.page] ?? final.pages[0], written);
      } catch (e) {
        this.thread = [...this.thread, { kind: "error", text: String(e) }];
      }
    }
    if (p.steps.some((s) => s.template)) void templates.refresh();
    editor.showProposal(null);
    editor.commit(final);
    this.#close("accepted");
  }

  refuse() {
    if (!this.proposal || this.busy) return;
    editor.showProposal(null);
    this.#close("refused");
  }

  /** Affiche le document juste après `step` ; null : la proposition entière, ou le document. */
  view(step: AssistantStep | null) {
    this.viewing = step;
    editor.showProposal(step ? step.document : this.proposal ? lastDoc(this.proposal) : null);
  }

  /** Avant d'enregistrer, d'imprimer ou de quitter : la proposition est gardée ou non. false : Annuler. */
  async settle(): Promise<boolean> {
    return (await this.#ask()) !== "cancel";
  }

  /** La modification à la main attendue pendant la question ; la dernière l'emporte (un glisser en fait plusieurs). */
  #edit: Document | null = null;

  /** Une modification à la main pendant la proposition : la question d'abord. Gardée, la modification s'applique
   *  par-dessus, si elle partait bien de la proposition entière (ni pendant le travail, ni sur une étape). */
  async editDuring(next: Document) {
    const asking = this.#edit !== null;
    this.#edit = next;
    if (asking) return;
    const onTop = !this.busy && !this.viewing;
    const choice = await this.#ask();
    const edit = this.#edit;
    this.#edit = null;
    if (choice === "accept" && onTop && edit) editor.commit(edit);
  }

  async #ask(): Promise<"cancel" | "accept" | "refuse" | "none"> {
    if (this.busy) {
      this.stop();
      await this.#running;
    }
    if (!this.proposal) {
      if (this.viewing) this.view(null);
      return "none";
    }
    const choice = await modal.choose<"cancel" | "refuse" | "accept">("L'assistant a proposé des modifications. Les garder ?", [
      { value: "cancel", label: "Annuler" },
      { value: "refuse", label: "Ne pas garder" },
      { value: "accept", label: "Garder", primary: true },
    ]);
    if (choice === "accept") await this.accept();
    else if (choice === "refuse") this.refuse();
    return choice;
  }

  /** Autre document : le fil et la proposition sont oubliés. */
  clear() {
    this.#history = [];
    this.thread = [];
    this.proposal = null;
    this.viewing = null;
    editor.showProposal(null);
  }
}

export const assistant = new Assistant();
editor.proposalGate = { edit: (next) => void assistant.editDuring(next), undo: () => assistant.refuse() };
