// Classe ouverte et liste des classes du dossier. Chaque modification s'enregistre seule, un peu après
// (un enregistrement à la fois). Le chemin de la dernière classe est gardé dans les préférences du poste.
import { classCreate, classImportPhoto, classList, classOpen, classSave } from "./api";
import { matchPhotos } from "./classFill";
import { editor } from "./docStore.svelte";
import { readPref, writePref } from "./prefs";
import type { AssetMeta, Class, ClassFileInfo, Level, OpenedClass, Student } from "./types";

const SAVE_DELAY_MS = 500;

/** Élèves d'un niveau ou d'un groupe ; sans critère, tous. */
export function filterStudents(students: Student[], f: { level?: Level | null; group?: string | null }): Student[] {
  return students.filter((s) => (!f.level || s.level === f.level) && (!f.group || s.group === f.group));
}

/** Groupes de la classe, une fois chacun, triés. */
export function groups(students: Student[]): string[] {
  return [...new Set(students.map((s) => s.group.trim()).filter(Boolean))].sort((a, b) => a.localeCompare(b, "fr"));
}

/** Année scolaire d'une date : elle change le 1er août. */
export function schoolYear(d: Date): string {
  const start = d.getMonth() >= 7 ? d.getFullYear() : d.getFullYear() - 1;
  return `${start}-${start + 1}`;
}

class Classes {
  current = $state<OpenedClass | null>(null);
  list = $state.raw<ClassFileInfo[]>([]);
  #timer: ReturnType<typeof setTimeout> | null = null;
  #saving: Promise<void> | null = null;
  #dirty = false;
  /** Des modifications attendent ou n'ont pas pu être enregistrées. */
  unsaved = $state(false);

  async refresh() {
    try {
      this.list = await classList();
    } catch (e) {
      editor.notice = String(e);
    }
  }

  #opened(c: OpenedClass) {
    editor.registerAssets(Object.values(c.class.photos).filter((m) => !!m));
    this.current = c;
    writePref("lastClass", c.path);
  }

  async create(name: string, year: string) {
    try {
      this.#opened(await classCreate(name, year));
      await this.refresh();
    } catch (e) {
      editor.notice = String(e);
    }
  }

  /**
   * Photos de plusieurs enfants d'un coup : chacune va à l'enfant dont elle porte le prénom. `consent` pose
   * une seule fois la question de l'autorisation ; sans réponse positive, chaque case reste à cocher.
   */
  async addPhotos(paths: string[], consent: (n: number) => Promise<boolean>): Promise<{ added: number; missed: number }> {
    if (!this.current) return { added: 0, missed: paths.length };
    const { matched, unmatched } = matchPhotos(this.current.class.students, paths);
    const metas = new Map<string, AssetMeta>();
    let missed = unmatched.length;
    for (const m of matched) {
      try {
        metas.set(m.student, await classImportPhoto(m.path));
      } catch {
        missed++;
      }
    }
    if (metas.size === 0) return { added: 0, missed };
    editor.registerAssets([...metas.values()]);
    const allowed = await consent(metas.size);
    this.update((c) => ({
      ...c,
      photos: { ...c.photos, ...Object.fromEntries([...metas.values()].map((m) => [m.id, m])) },
      students: c.students.map((s) => (metas.has(s.id) ? { ...s, photo: metas.get(s.id)!.id, photo_allowed: allowed } : s)),
    }));
    return { added: metas.size, missed };
  }

  /** Nouvelle classe sous ce nom, ou « nom 2 », « nom 3 »… si le nom est déjà pris cette année. */
  async createNamed(name: string, year: string) {
    const taken = new Set(this.list.filter((c) => c.year === year).map((c) => c.name));
    let n = 1;
    while (taken.has(n === 1 ? name : `${name} ${n}`)) n++;
    await this.create(n === 1 ? name : `${name} ${n}`, year);
  }

  async open(path: string) {
    await this.flush();
    // Une classe qui n'a pas pu être enregistrée reste ouverte : ses modifications ne sont pas perdues.
    if (this.unsaved) return;
    try {
      this.#opened(await classOpen(path));
    } catch (e) {
      editor.notice = String(e);
    }
  }

  /** Rouvre la dernière classe, si son fichier est toujours là. */
  async restore() {
    const path = readPref<string | null>("lastClass", null);
    if (!path) return;
    try {
      this.#opened(await classOpen(path));
    } catch {
      writePref("lastClass", null);
    }
  }

  async close() {
    await this.flush();
    if (this.unsaved) return;
    this.current = null;
    writePref("lastClass", null);
    await this.refresh();
  }

  update(fn: (c: Class) => Class) {
    if (!this.current) return;
    this.current = { ...this.current, class: fn(this.current.class) };
    this.#dirty = true;
    this.unsaved = true;
    if (this.#timer) clearTimeout(this.#timer);
    this.#timer = setTimeout(() => void this.flush(), SAVE_DELAY_MS);
  }

  /** Enregistre maintenant ce qui attend ; un seul enregistrement en vol. */
  async flush() {
    if (this.#timer) clearTimeout(this.#timer);
    this.#timer = null;
    while (this.#saving) await this.#saving;
    if (!this.#dirty || !this.current) return;
    this.#dirty = false;
    const { path, class: cls } = this.current;
    this.#saving = classSave(path, cls)
      .then(() => void (this.unsaved = this.#dirty))
      .catch((e) => {
        this.#dirty = true;
        editor.notice = String(e);
      })
      .finally(() => (this.#saving = null));
    await this.#saving;
  }
}

export const classes = new Classes();
