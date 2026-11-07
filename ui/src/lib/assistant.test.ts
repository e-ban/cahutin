import { beforeEach, expect, test, vi } from "vitest";
import type { AssistantEvent, AssistantRequest, Document } from "./types";

// Le modèle simulé : chaque envoi rejoue la prochaine liste d'événements.
const script: ((req: AssistantRequest) => AssistantEvent[])[] = [];
const sent: AssistantRequest[] = [];
/** Comme Tauri pour les gros messages : les événements arrivent après la fin de l'appel. */
let late = false;
const saved: { name: string; keep?: string[] }[] = [];
const written: Document[] = [];
const reports: unknown[][] = [];
/** Le dernier envoi : pour finir à la main une demande restée en cours. */
let emit: (e: AssistantEvent) => void = () => {};

vi.mock("./api", () => ({
  assistantSend: async (request: AssistantRequest, onEvent: (e: AssistantEvent) => void) => {
    emit = onEvent;
    sent.push(request);
    const events = script.shift()?.(request) ?? [];
    if (late) setTimeout(() => events.forEach(onEvent), 20);
    else for (const e of events) onEvent(e);
  },
  assistantStop: async () => {},
  assistantReport: async (...args: unknown[]) => (reports.push(args), "auteur@exemple.fr"),
  assistantSettings: async () => ({ enabled: true, url: "http://localhost:11434/v1", model: "qwen3:4b", has_key: false }),
  savePageAsTemplate: async (name: string, _page: unknown, keep?: string[]) => void saved.push({ name, keep }),
  listTemplates: async () => [],
  saveDocument: async (_path: string, doc: Document) => (written.push(doc), null),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn(), ask: vi.fn() }));

const { assistant } = await import("./assistant.svelte");
const { editor } = await import("./docStore.svelte");
const { modal } = await import("./modal.svelte");
const { newDocument } = await import("./factory");
const { fr } = await import("./i18n/fr");

const titled = (d: Document, title: string): Document => ({ ...d, title });

function step(doc: Document, extra: Partial<Extract<AssistantEvent, { type: "Step" }>> = {}): AssistantEvent {
  return { type: "Step", document: doc, summary: ["Texte : déplacé"], touched: [], written: [], template: null, ...extra };
}
const done = (reply = "C'est fait.", error: string | null = null, history: unknown[] = [{ role: "user", content: "x" }]): AssistantEvent => ({
  type: "Done",
  reply,
  history,
  error,
});

beforeEach(() => {
  script.length = sent.length = saved.length = written.length = reports.length = 0;
  late = false;
  editor.reset(newDocument(), null);
  assistant.clear();
});

test("steps_preview_without_commit", async () => {
  script.push((r) => [step(titled(r.document, "A")), step(titled(r.document, "B")), done()]);
  await assistant.send("Change le titre");
  expect(editor.doc.title).toBe("B");
  expect(editor.canUndo).toBe(false);
  expect(editor.dirty).toBe(false);
  expect(assistant.proposal?.steps.length).toBe(2);
});

test("accept_is_one_undo", async () => {
  const base = editor.doc;
  script.push((r) => [step(titled(r.document, "A")), step(titled(r.document, "B")), done()]);
  await assistant.send("x");
  await assistant.accept();
  expect(editor.doc.title).toBe("B");
  expect(assistant.proposal).toBeNull();
  editor.undo();
  expect(editor.doc).toBe(base);
  const last = assistant.thread.at(-1);
  expect(last?.kind === "assistant" && last.outcome).toBe("accepted");
});

test("refuse_restores_the_base", async () => {
  const base = editor.doc;
  script.push((r) => [step(titled(r.document, "A")), done()]);
  await assistant.send("x");
  assistant.refuse();
  expect(editor.doc).toBe(base);
  expect(editor.proposing).toBe(false);
  const last = assistant.thread.at(-1);
  expect(last?.kind === "assistant" && last.outcome).toBe("refused");
});

const settled = () => new Promise((r) => setTimeout(r, 0));

test("undo during a proposal refuses it", async () => {
  const base = editor.doc;
  script.push((r) => [step(titled(r.document, "A")), done()]);
  await assistant.send("x");
  editor.undo();
  expect(editor.doc).toBe(base);
  expect(editor.proposing).toBe(false);
  const last = assistant.thread.at(-1);
  expect(last?.kind === "assistant" && last.outcome).toBe("refused");
});

test("a manual edit during a proposal asks first; kept, the edit applies on top, one undo each", async () => {
  const base = editor.doc;
  script.push((r) => [step(titled(r.document, "A")), done()]);
  await assistant.send("x");
  editor.commit(titled(editor.doc, "à la main"));
  editor.commit(titled(editor.doc, "à la main 2"));
  await Promise.resolve();
  expect(modal.message).toBe("L'assistant a proposé des modifications. Les garder ?");
  expect(editor.doc.title).toBe("A");
  modal.answer("accept");
  await settled();
  expect(editor.doc.title).toBe("à la main 2");
  editor.undo();
  expect(editor.doc.title).toBe("A");
  editor.undo();
  expect(editor.doc).toBe(base);
});

test("a manual edit while the assistant works neither stops it nor asks: a message says to wait", async () => {
  script.push((r) => [step(titled(r.document, "A"))]);
  const sending = assistant.send("x");
  await Promise.resolve();
  expect([assistant.busy, editor.proposing]).toEqual([true, true]);
  modal.message = null;
  editor.commit(titled(editor.doc, "à la main"));
  await Promise.resolve();
  expect([assistant.stopping, modal.message]).toEqual([false, null]);
  expect(editor.notice).toBe(fr.assistant.waitWorking);
  emit(done());
  await sending;
  expect(editor.doc.title).toBe("A");
});

test("a manual edit during a proposal: not kept, the page goes back and the edit is dropped; cancel keeps it", async () => {
  const base = editor.doc;
  script.push((r) => [step(titled(r.document, "A")), done()]);
  await assistant.send("x");
  editor.commit(titled(editor.doc, "à la main"));
  await Promise.resolve();
  modal.answer("cancel");
  await settled();
  expect([editor.doc.title, assistant.proposal !== null]).toEqual(["A", true]);
  editor.commit(titled(editor.doc, "à la main"));
  await Promise.resolve();
  modal.answer("refuse");
  await settled();
  expect(editor.doc).toBe(base);
  expect(editor.proposing).toBe(false);
});

test("follow_up_continues_the_proposal", async () => {
  const history = [{ role: "user", content: "premier" }, { role: "assistant", content: "ok" }];
  script.push((r) => [step(titled(r.document, "A")), done("ok", null, history)]);
  script.push((r) => [step(titled(r.document, r.document.title + "+")), done()]);
  await assistant.send("premier");
  await assistant.send("encore");
  expect(sent[1].history).toEqual(history);
  expect(sent[1].document.title).toBe("A");
  expect(editor.doc.title).toBe("A+");
  expect(assistant.proposal?.steps.length).toBe(2);
});

test("a reply without steps makes no proposal", async () => {
  script.push(() => [done("Je ne vois pas de titre.")]);
  await assistant.send("x");
  expect(assistant.proposal).toBeNull();
  expect(editor.proposing).toBe(false);
  const last = assistant.thread.at(-1);
  expect(last?.kind === "assistant" && last.outcome).toBe("none");
});

test("error_keeps_the_proposal", async () => {
  script.push((r) => [step(titled(r.document, "A")), done("", "Je n'arrive pas à joindre l'assistant.")]);
  await assistant.send("x");
  expect(assistant.proposal?.steps.length).toBe(1);
  expect(editor.doc.title).toBe("A");
  expect(assistant.thread.at(-1)).toEqual({ kind: "error", text: "Je n'arrive pas à joindre l'assistant." });
});

test("an error with nothing done shows the error only, not « no change »", async () => {
  script.push(() => [done("", "Je n'arrive pas à joindre l'assistant.")]);
  await assistant.send("x");
  expect(assistant.thread.map((e) => e.kind)).toEqual(["user", "error"]);
});

test("viewing a step shows its document, then back to the end", async () => {
  script.push((r) => [step(titled(r.document, "A")), step(titled(r.document, "B")), done()]);
  await assistant.send("x");
  const first = assistant.proposal!.steps[0];
  assistant.view(first);
  expect(editor.doc.title).toBe("A");
  expect(assistant.viewing).toBe(first);
  assistant.view(null);
  expect(editor.doc.title).toBe("B");
});

test("accept_saves_requested_templates_with_written_ids", async () => {
  script.push((r) => [
    step(titled(r.document, "A"), { written: ["t1"] }),
    step(titled(r.document, "A"), { template: { name: "Saisons", page: 0 } }),
    step(titled(r.document, "B"), { written: ["t2"] }),
    done(),
  ]);
  await assistant.send("x");
  await assistant.accept();
  expect(saved).toEqual([{ name: "Saisons", keep: ["t1", "t2"] }]);
});

test("copies_of_written_texts_are_kept_in_the_template_but_not_copies_of_the_teachers", async () => {
  const text = (id: string, words: string) => ({
    id,
    frame: { x_mm: 10, y_mm: 10, w_mm: 50, h_mm: 10, rotation_deg: 0 },
    locked: false,
    zone: false,
    kind: { type: "Text" as const, content: { paragraphs: [{ align: null, runs: [{ text: words, bold: false, italic: false, underline: false, color: null, font: null, size_pt: null }] }] }, style: { font: "Andika", size_pt: 14, color: "#222222", align: "Left" as const }, prompt: null },
  });
  const base = editor.doc;
  const withTexts = (d: Document, els: ReturnType<typeof text>[]): Document => ({ ...d, pages: [{ ...d.pages[0], elements: [...d.pages[0].elements, ...els] }, ...d.pages.slice(1)] });
  const teacher = withTexts(base, [text("u1", "Mme Durand")]);
  editor.reset(teacher, null);
  script.push((r) => {
    const one = withTexts(r.document, [text("t1", "Bon anniversaire {prénom}")]);
    // « Répéter » : des copies du texte de l'assistant et du texte de l'enseignante.
    const many = withTexts(one, [text("t1b", "Bon anniversaire {prénom}"), text("u1b", "Mme Durand")]);
    return [step(one, { written: ["t1"] }), step(many), step(many, { template: { name: "Anniversaires", page: 0 } }), done()];
  });
  await assistant.send("x");
  await assistant.accept();
  expect(saved).toEqual([{ name: "Anniversaires", keep: ["t1", "t1b"] }]);
});

test("save_settles_the_proposal_first", async () => {
  const { save } = await import("./fileActions");
  editor.markSaved("/tmp/essai.carnet");
  const base = editor.doc;
  script.push((r) => [step(titled(r.document, "A")), done()]);
  await assistant.send("x");
  const saving = save();
  await Promise.resolve();
  expect(modal.message).toBe("L'assistant a proposé des modifications. Les garder ?");
  modal.answer("refuse");
  expect(await saving).toBe(true);
  expect(written.map((d) => d.title)).toEqual([base.title]);
  expect(written[0].pages).toBe(base.pages);
});

test("settle cancel keeps the proposal and stops the save", async () => {
  const { save } = await import("./fileActions");
  editor.markSaved("/tmp/essai.carnet");
  script.push((r) => [step(titled(r.document, "A")), done()]);
  await assistant.send("x");
  const saving = save();
  await Promise.resolve();
  modal.answer("cancel");
  expect(await saving).toBe(false);
  expect(written).toEqual([]);
  expect(assistant.proposal).not.toBeNull();
});

test("events arriving after the call has returned are not lost", async () => {
  late = true;
  script.push((r) => [step(titled(r.document, "A")), done("Fini.")]);
  await assistant.send("x");
  expect(assistant.busy).toBe(false);
  const last = assistant.thread.at(-1);
  expect(last?.kind === "assistant" && last.text).toBe("Fini.");
  expect(editor.doc.title).toBe("A");
});

test("the page follows the change", async () => {
  script.push((r) => {
    const page2 = { ...r.document.pages[0], id: "p2", elements: [] };
    return [step({ ...r.document, pages: [...r.document.pages, page2] }), done()];
  });
  await assistant.send("ajoute une page");
  expect(editor.pageIndex).toBe(1);
});

test("a reply without edits keeps the conversation for the next request", async () => {
  const history = [{ role: "user", content: "mets le titre en rouge" }, { role: "assistant", content: "Quel titre ?" }];
  script.push(() => [done("Quel titre ?", null, history)]);
  script.push((r) => [step(titled(r.document, "A")), done()]);
  await assistant.send("mets le titre en rouge");
  await assistant.send("le premier");
  expect(sent[1].history).toEqual(history);
});

test("after accepting, the next request starts a fresh conversation", async () => {
  script.push((r) => [step(titled(r.document, "A")), done("ok", null, [{ role: "user", content: "x" }])]);
  script.push(() => [done()]);
  await assistant.send("x");
  await assistant.accept();
  await assistant.send("autre chose");
  expect(sent[1].history).toEqual([]);
});

test("turning the assistant off drops the proposal and frees the page", async () => {
  script.push((r) => [step(titled(r.document, "A")), done()]);
  await assistant.send("x");
  assistant.turnedOff();
  expect(assistant.proposal).toBeNull();
  expect(editor.proposing).toBe(false);
  expect(assistant.open).toBe(false);
});

test("saving the page as a template settles the proposal first", async () => {
  const { templates } = await import("./templates.svelte");
  script.push((r) => [step(titled(r.document, "A")), done()]);
  await assistant.send("x");
  const saving = templates.saveCurrentPage("Mon modèle");
  await Promise.resolve();
  expect(modal.message).toBe("L'assistant a proposé des modifications. Les garder ?");
  modal.answer("cancel");
  await saving;
  expect(saved).toEqual([]);
});

test("a reply is reported by email with its request", async () => {
  script.push((r) => [step(titled(r.document, "A")), done("Voilà un titre.")]);
  await assistant.send("Mets un titre");
  await assistant.report(assistant.thread.at(-1)!);
  expect(reports).toEqual([["Mets un titre", "Voilà un titre.", ["Texte : déplacé"]]]);
  expect(editor.notice).toContain("auteur@exemple.fr");
});
