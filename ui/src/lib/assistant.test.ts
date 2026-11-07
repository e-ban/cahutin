import { beforeEach, expect, test, vi } from "vitest";
import type { AssistantEvent, AssistantRequest, Document } from "./types";

// Le modèle simulé : chaque envoi rejoue la prochaine liste d'événements.
const script: ((req: AssistantRequest) => AssistantEvent[])[] = [];
const sent: AssistantRequest[] = [];
/** Comme Tauri pour les gros messages : les événements arrivent après la fin de l'appel. */
let late = false;
const saved: { name: string; keep?: string[] }[] = [];
const written: Document[] = [];

vi.mock("./api", () => ({
  assistantSend: async (request: AssistantRequest, onEvent: (e: AssistantEvent) => void) => {
    sent.push(request);
    const events = script.shift()?.(request) ?? [];
    if (late) setTimeout(() => events.forEach(onEvent), 20);
    else for (const e of events) onEvent(e);
  },
  assistantStop: async () => {},
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
  script.length = sent.length = saved.length = written.length = 0;
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

test("manual_edit_is_ignored_during_a_proposal", async () => {
  script.push((r) => [step(titled(r.document, "A")), done()]);
  await assistant.send("x");
  editor.commit(titled(editor.doc, "à la main"));
  expect(editor.doc.title).toBe("A");
  editor.undo();
  expect(editor.doc.title).toBe("A");
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
