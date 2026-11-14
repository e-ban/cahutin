import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AUTOSAVE_DELAY_MS, AUTOSAVE_MAX_WAIT_MS, AUTOSAVE_TICK_MS, Autosaver, saveStatus, type AutosaveProblem, type AutosaveView } from "./autosave";

type Doc = { n: number };

/** Un faux éditeur : `change()` valide un nouveau document, `write` réussit, échoue ou trouve le fichier changé. */
function setup(view: Partial<AutosaveView<Doc>> = {}) {
  const state: AutosaveView<Doc> = { enabled: true, path: "/docs/fiche.carnet", dirty: false, doc: { n: 0 }, generation: 1, blocked: false, ...view };
  let saved = state.doc;
  const writes: Doc[] = [];
  const problems: AutosaveProblem[] = [];
  let result: "saved" | "changed" | Error = "saved";
  let duration = 0;
  const saver = new Autosaver<Doc>({
    view: () => ({ ...state, dirty: state.doc !== saved }),
    write: async (_path, doc) => {
      if (duration) await new Promise((r) => setTimeout(r, duration));
      if (result instanceof Error) throw result.message;
      if (result === "changed") return "changed";
      writes.push(doc);
      saved = doc;
      return "saved";
    },
    problem: (p) => problems.push(p),
    now: () => Date.now(),
  });
  const stop = saver.start();
  return {
    state,
    saver,
    writes,
    problems,
    stop,
    change() {
      state.doc = { n: state.doc.n + 1 };
    },
    fail(message = "Disque plein") {
      result = new Error(message);
    },
    succeed() {
      result = "saved";
    },
    changedElsewhere() {
      result = "changed";
    },
    slow(ms: number) {
      duration = ms;
    },
  };
}

const wait = (ms: number) => vi.advanceTimersByTimeAsync(ms);

describe("Autosaver", () => {
  let t: ReturnType<typeof setup>;
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => {
    t?.stop();
    vi.useRealTimers();
  });

  it("writes the validated document a few seconds after the last change", async () => {
    t = setup();
    t.change();
    await wait(AUTOSAVE_DELAY_MS - AUTOSAVE_TICK_MS);
    expect(t.writes).toEqual([]);
    await wait(2 * AUTOSAVE_TICK_MS);
    expect(t.writes).toEqual([t.state.doc]);
    // Rien de nouveau : rien n'est écrit de nouveau.
    await wait(AUTOSAVE_MAX_WAIT_MS);
    expect(t.writes.length).toBe(1);
  });

  it("each change restarts the delay, but a run of changes is written after the maximum wait", async () => {
    t = setup();
    for (let elapsed = 0; elapsed < AUTOSAVE_MAX_WAIT_MS - 2 * AUTOSAVE_TICK_MS; elapsed += 2 * AUTOSAVE_TICK_MS) {
      t.change();
      await wait(2 * AUTOSAVE_TICK_MS);
      expect(t.writes).toEqual([]);
    }
    t.change();
    await wait(3 * AUTOSAVE_TICK_MS);
    expect(t.writes.length).toBe(1);
  });

  it("never writes while something is going on (text, gesture, preview, crop, proposal, save), even past the maximum wait", async () => {
    t = setup();
    t.change();
    t.state.blocked = true;
    await wait(2 * AUTOSAVE_MAX_WAIT_MS);
    expect(t.writes).toEqual([]);
    t.state.blocked = false;
    await wait(AUTOSAVE_TICK_MS);
    expect(t.writes.length).toBe(1);
  });

  it("never writes a document without a file of the teacher's", async () => {
    t = setup({ path: null });
    t.change();
    await wait(2 * AUTOSAVE_MAX_WAIT_MS);
    expect(t.writes).toEqual([]);
  });

  it("never writes when the setting is off; once checked, waits the normal delay", async () => {
    t = setup({ enabled: false });
    t.change();
    await wait(2 * AUTOSAVE_MAX_WAIT_MS);
    expect(t.writes).toEqual([]);
    expect(await t.saver.flush()).toBe(false);
    expect(t.writes).toEqual([]);
    t.state.enabled = true;
    await wait(AUTOSAVE_DELAY_MS - AUTOSAVE_TICK_MS);
    expect(t.writes).toEqual([]);
    await wait(2 * AUTOSAVE_TICK_MS);
    expect(t.writes.length).toBe(1);
  });

  it("a failure is told once, and retried at the next change without a new notice", async () => {
    t = setup();
    t.fail("Ce fichier est en lecture seule");
    t.change();
    await wait(AUTOSAVE_DELAY_MS + AUTOSAVE_TICK_MS);
    expect(t.problems).toEqual([{ kind: "failed", message: "Ce fichier est en lecture seule" }]);
    // Pas de nouvelle tentative sans changement.
    await wait(2 * AUTOSAVE_MAX_WAIT_MS);
    t.change();
    await wait(AUTOSAVE_DELAY_MS + AUTOSAVE_TICK_MS);
    expect(t.problems.length).toBe(1);
    expect(t.writes).toEqual([]);
    // La cause a disparu : écrit au changement suivant, après le délai normal (pas plus tôt, même longtemps
    // après l'échec), et le problème est fini.
    t.succeed();
    await wait(2 * AUTOSAVE_MAX_WAIT_MS);
    t.change();
    await wait(AUTOSAVE_DELAY_MS - AUTOSAVE_TICK_MS);
    expect(t.writes).toEqual([]);
    await wait(2 * AUTOSAVE_TICK_MS);
    expect(t.writes).toEqual([t.state.doc]);
    expect(t.problems).toEqual([{ kind: "failed", message: "Ce fichier est en lecture seule" }, null]);
  });

  it("a file changed elsewhere is never overwritten: told once, then nothing until saved by hand", async () => {
    t = setup();
    t.changedElsewhere();
    t.change();
    await wait(AUTOSAVE_DELAY_MS + AUTOSAVE_TICK_MS);
    expect(t.problems).toEqual([{ kind: "changed" }]);
    t.succeed();
    t.change();
    await wait(2 * AUTOSAVE_MAX_WAIT_MS);
    expect(t.writes).toEqual([]);
    expect(await t.saver.flush()).toBe(false);
    // « Enregistrer une copie » ou « Remplacer » : le problème est réglé, l'enregistrement reprend.
    t.saver.cleared();
    t.change();
    await wait(AUTOSAVE_DELAY_MS + AUTOSAVE_TICK_MS);
    expect(t.writes.length).toBe(1);
  });

  it("another document starts without the problems of the previous one", async () => {
    t = setup();
    t.changedElsewhere();
    t.change();
    await wait(AUTOSAVE_DELAY_MS + AUTOSAVE_TICK_MS);
    t.succeed();
    t.state.generation = 2;
    t.change();
    await wait(AUTOSAVE_DELAY_MS + AUTOSAVE_TICK_MS);
    expect(t.problems).toEqual([{ kind: "changed" }, null]);
    expect(t.writes.length).toBe(1);
  });

  it("leaving writes at once, after the write in progress", async () => {
    t = setup();
    t.slow(3000);
    t.change();
    await wait(AUTOSAVE_DELAY_MS + AUTOSAVE_TICK_MS);
    t.change();
    const leaving = t.saver.flush();
    await wait(6000);
    expect(await leaving).toBe(true);
    expect(t.writes).toEqual([{ n: 1 }, { n: 2 }]);
  });

  it("one write at a time; what changed meanwhile waits the delay after the write ends", async () => {
    t = setup();
    t.slow(10_000);
    t.change();
    await wait(AUTOSAVE_DELAY_MS + AUTOSAVE_TICK_MS);
    t.change();
    await wait(10_000);
    expect(t.writes).toEqual([{ n: 1 }]);
    await wait(AUTOSAVE_DELAY_MS - AUTOSAVE_TICK_MS);
    expect(t.writes.length).toBe(1);
    await wait(2 * AUTOSAVE_TICK_MS + 10_000);
    expect(t.writes).toEqual([{ n: 1 }, { n: 2 }]);
  });

  it("leaving with a failing write says so (the old question comes back)", async () => {
    t = setup();
    t.fail();
    t.change();
    expect(await t.saver.flush()).toBe(false);
    expect(t.problems.length).toBe(1);
  });
});

describe("saveStatus", () => {
  const base = { enabled: true, path: "/docs/fiche.carnet", ownFile: true, dirty: false, editing: false, problem: null };
  it("says saved, saving or not saved", () => {
    expect(saveStatus(base)).toBe("saved");
    expect(saveStatus({ ...base, dirty: true })).toBe("saving");
    expect(saveStatus({ ...base, path: null })).toBe("unsaved");
    expect(saveStatus({ ...base, dirty: true, problem: { kind: "changed" } })).toBe("unsaved");
    expect(saveStatus({ ...base, dirty: true, problem: { kind: "failed", message: "x" } })).toBe("unsaved");
    // Réglage décoché : comme avant, modifié veut dire non enregistré.
    expect(saveStatus({ ...base, enabled: false, dirty: true })).toBe("unsaved");
    expect(saveStatus({ ...base, enabled: false })).toBe("saved");
    // Un texte en cours d'écriture (ou un recadrage) : pas encore écrit, il le sera à la sortie du bloc.
    expect(saveStatus({ ...base, editing: true })).toBe("saving");
    expect(saveStatus({ ...base, editing: true, enabled: false })).toBe("saved");
    // Copie de secours reprise : son fichier d'origine n'est pas écrit tout seul.
    expect(saveStatus({ ...base, ownFile: false, dirty: true })).toBe("unsaved");
  });
});
