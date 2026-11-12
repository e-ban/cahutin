// Photothèque de la classe, côté interface : périodes, filtres, comptes par élève, équité des traces et
// autorisations photo. Le moteur range les photos ; ici, rien que des calculs sur l'index.
import { normalize } from "./fontSearch";
import type { ClassPhoto, Student } from "./types";

export type Period = "month" | "trimester" | "year" | "all";

/** Premier jour de la période, « AAAA-MM-JJ » ; l'année scolaire commence le 1er septembre. */
export function periodStart(period: Period, today: Date): string | null {
  const iso = (d: Date) => d.toISOString().slice(0, 10);
  const y = today.getUTCFullYear();
  const m = today.getUTCMonth();
  switch (period) {
    case "month":
      return iso(new Date(Date.UTC(y, m, 1)));
    case "trimester":
      return iso(new Date(Date.UTC(y, m - 2, 1)));
    case "year":
      return iso(new Date(Date.UTC(m >= 8 ? y : y - 1, 8, 1)));
    case "all":
      return null;
  }
}

/** Jour retenu pour une photo : la prise de vue, sinon l'ajout. */
export const dayOf = (p: ClassPhoto) => p.taken ?? p.added;

export interface LibraryFilter {
  period: Period;
  /** Un élève : ses photos seulement. */
  student?: string | null;
  /** Une activité. */
  tag?: string | null;
  /** Seulement les photos où aucun élève n'est indiqué (le tri à faire). */
  untagged?: boolean;
  /** Un dossier (celui de la photo, `ClassPhoto.folder`). */
  folder?: string | null;
}

/** Les photos retenues, de la plus récente à la plus ancienne. */
export function filterLibrary(photos: ClassPhoto[], f: LibraryFilter, today: Date): ClassPhoto[] {
  const start = periodStart(f.period, today);
  return photos
    .filter((p) => !start || dayOf(p) >= start)
    .filter((p) => !f.student || p.students.includes(f.student))
    .filter((p) => !f.tag || p.tags.includes(f.tag))
    .filter((p) => !f.untagged || p.students.length === 0)
    .filter((p) => !f.folder || p.folder === f.folder)
    .sort((a, b) => dayOf(b).localeCompare(dayOf(a)));
}

/** Les photos montrées : ni retirées (« Retirer »), ni absentes (clé USB débranchée, fichier déplacé hors des
 *  dossiers suivis). Les autres restent dans l'index, avec leurs élèves et activités. */
export const visiblePhotos = (photos: ClassPhoto[]) => photos.filter((p) => !p.hidden && !p.missing);

/** Nombre de photos de chaque dossier, par ordre alphabétique. */
export function folderCounts(photos: ClassPhoto[]): { folder: string; count: number }[] {
  const n = new Map<string, number>();
  for (const p of photos) n.set(p.folder, (n.get(p.folder) ?? 0) + 1);
  return [...n.entries()].sort(([a], [b]) => a.localeCompare(b, "fr")).map(([folder, count]) => ({ folder, count }));
}

/** Un lot de photos reçu pendant un parcours : celles déjà là sont remplacées (même identifiant, même place),
 *  les nouvelles ajoutées à la fin. */
export function upsertPhotos(photos: ClassPhoto[], batch: ClassPhoto[]): ClassPhoto[] {
  const incoming = new Map(batch.map((p) => [p.id, p]));
  const out = photos.map((p) => {
    const next = incoming.get(p.id);
    if (!next) return p;
    incoming.delete(p.id);
    return next;
  });
  return [...out, ...incoming.values()];
}

/** Version du fichier d'une photo, pour son adresse : une photo changée sur le disque est rechargée. */
export const photoVersion = (p: ClassPhoto) => `${p.size}-${p.modified}`;

/** Le dernier nom d'un chemin (dossier ou fichier), avec « / » comme avec « \ ». */
export const baseName = (path: string) => path.split(/[\\/]+/).filter(Boolean).at(-1) ?? path;

/** Une copie faite par une version précédente : le fichier porte pour nom l'identifiant de la photo. */
export function isOldCopy(p: ClassPhoto): boolean {
  const name = baseName(p.path);
  const dot = name.lastIndexOf(".");
  return dot > 0 && dot < name.length - 1 && name.slice(0, dot) === p.id;
}

/** Les anciennes copies encore montrées : celles dont il reste à retrouver l'original. */
export const oldCopies = (photos: ClassPhoto[]) => visiblePhotos(photos).filter(isOldCopy);

/** Nombre de photos de chaque élève sur la période. */
export function countsByStudent(photos: ClassPhoto[], students: Student[], period: Period, today: Date): Map<string, number> {
  const inPeriod = filterLibrary(photos, { period }, today);
  return new Map(students.map((s) => [s.id, inPeriod.filter((p) => p.students.includes(s.id)).length]));
}

/**
 * Élèves peu photographiés sur la période, du moins au plus : moins de la moitié de la médiane de la classe
 * (et au plus une photo s'ils sont peu nombreux). Rien tant que la classe a trop peu de photos pour comparer.
 */
export function studentsBehind(counts: Map<string, number>): string[] {
  const values = [...counts.values()].sort((a, b) => a - b);
  if (values.length === 0) return [];
  const median = values[Math.floor(values.length / 2)];
  if (median < 2) return [];
  const limit = Math.max(1, Math.floor(median / 2));
  return [...counts.entries()].filter(([, n]) => n <= limit).sort((a, b) => a[1] - b[1]).map(([id]) => id);
}

/** Les activités de la photothèque, des plus utilisées aux moins utilisées. */
export function tagsOf(photos: ClassPhoto[]): string[] {
  const n = new Map<string, number>();
  for (const t of photos.flatMap((p) => p.tags)) n.set(t, (n.get(t) ?? 0) + 1);
  return [...n.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0], "fr")).map(([t]) => t);
}

/** Les élèves de la photo dont les parents n'ont pas donné l'autorisation : la photo est à flouter avant de servir. */
export function withoutConsent(p: ClassPhoto, students: Student[]): Student[] {
  return students.filter((s) => p.students.includes(s.id) && !s.photo_allowed);
}

/** Les élèves sans autorisation présents sur au moins une des photos `ids`, chacun une fois. */
export function missingConsent(photos: ClassPhoto[], students: Student[], ids: string[]): Student[] {
  const shown = photos.filter((p) => ids.includes(p.id));
  return students.filter((s) => !s.photo_allowed && shown.some((p) => p.students.includes(s.id)));
}

/** Pour chaque élève, présent sur toutes les photos choisies, sur une partie, ou sur aucune. */
export function presence(photos: ClassPhoto[], studentId: string): "all" | "some" | "none" {
  const n = photos.filter((p) => p.students.includes(studentId)).length;
  return n === 0 ? "none" : n === photos.length ? "all" : "some";
}

/**
 * Le nom affiché de chaque élève : son prénom, suivi de l'initiale du nom quand un autre élève porte le
 * même prénom (sans accents ni capitales), du nom entier si l'initiale ne suffit pas, d'un numéro sinon.
 */
export function displayNames(students: Student[]): Map<string, string> {
  const twins = new Map<string, Student[]>();
  for (const s of students) twins.set(normalize(s.first_name), [...(twins.get(normalize(s.first_name)) ?? []), s]);
  const names = new Map<string, string>();
  for (const same of twins.values()) {
    same.forEach((s, i) => {
      const last = s.last_name.trim();
      const alone = (f: (x: Student) => string) => !!f(s) && same.filter((x) => f(x) === f(s)).length === 1;
      if (same.length === 1) names.set(s.id, s.first_name);
      else if (alone((x) => normalize(x.last_name).charAt(0))) names.set(s.id, `${s.first_name} ${last.charAt(0).toUpperCase()}.`);
      else if (alone((x) => normalize(x.last_name))) names.set(s.id, `${s.first_name} ${last}`);
      else names.set(s.id, `${s.first_name} (${i + 1})`);
    });
  }
  return names;
}

/** Les photos (déjà dans l'ordre) regroupées par jour de prise de vue. */
export function byDay(photos: ClassPhoto[]): { day: string; photos: ClassPhoto[] }[] {
  const days: { day: string; photos: ClassPhoto[] }[] = [];
  for (const p of photos) {
    const last = days.at(-1);
    if (last?.day === dayOf(p)) last.photos.push(p);
    else days.push({ day: dayOf(p), photos: [p] });
  }
  return days;
}
