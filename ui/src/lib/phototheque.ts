// Photothèque de la classe, côté interface : périodes, filtres, comptes par élève, équité des traces et
// autorisations photo. Le moteur range les photos ; ici, rien que des calculs sur l'index.
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
}

/** Les photos retenues, de la plus récente à la plus ancienne. */
export function filterLibrary(photos: ClassPhoto[], f: LibraryFilter, today: Date): ClassPhoto[] {
  const start = periodStart(f.period, today);
  return photos
    .filter((p) => !start || dayOf(p) >= start)
    .filter((p) => !f.student || p.students.includes(f.student))
    .filter((p) => !f.tag || p.tags.includes(f.tag))
    .filter((p) => !f.untagged || p.students.length === 0)
    .sort((a, b) => dayOf(b).localeCompare(dayOf(a)));
}

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

/** Pour chaque élève, présent sur toutes les photos choisies, sur une partie, ou sur aucune. */
export function presence(photos: ClassPhoto[], studentId: string): "all" | "some" | "none" {
  const n = photos.filter((p) => p.students.includes(studentId)).length;
  return n === 0 ? "none" : n === photos.length ? "all" : "some";
}
