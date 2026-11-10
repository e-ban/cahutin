import { expect, test } from "vitest";
import { filterStudents, groups, patchStudents, presetOf, presetStudents, schoolYear, withStudentBack, withoutStudent } from "./classStore.svelte";
import type { Student } from "./types";

const s = (first_name: string, level: Student["level"], group = ""): Student => ({
  id: first_name,
  first_name,
  last_name: "",
  level,
  group,
  photo: null,
  photo_allowed: false,
  symbol: null,
});

const list = [s("Léa", "MS", "Les loups"), s("Noé", "GS", "Les ours"), s("Anna", "MS", ""), s("Tom", null, "Les loups")];

test("filterStudents keeps a level, a group, or everyone", () => {
  expect(filterStudents(list, {}).map((x) => x.first_name)).toEqual(["Léa", "Noé", "Anna", "Tom"]);
  expect(filterStudents(list, { level: "MS" }).map((x) => x.first_name)).toEqual(["Léa", "Anna"]);
  expect(filterStudents(list, { group: "Les loups" }).map((x) => x.first_name)).toEqual(["Léa", "Tom"]);
  expect(filterStudents(list, { level: "PS" })).toEqual([]);
});

test("groups lists each group once, sorted, without the empty one", () => {
  expect(groups(list)).toEqual(["Les loups", "Les ours"]);
});

test("schoolYear changes on the first of August", () => {
  expect(schoolYear(new Date(2027, 6, 31))).toBe("2026-2027");
  expect(schoolYear(new Date(2027, 7, 1))).toBe("2027-2028");
});

test("withoutStudent takes one child out and remembers the place", () => {
  const r = withoutStudent(list, "Noé");
  expect(r.students.map((s) => s.first_name)).toEqual(["Léa", "Anna", "Tom"]);
  expect(r.removed?.first_name).toBe("Noé");
  expect(r.index).toBe(1);
  expect(withoutStudent(list, "absent").removed).toBeNull();
});

test("withStudentBack puts the child back where they were, or at the end", () => {
  const { students, removed, index } = withoutStudent(list, "Noé");
  expect(withStudentBack(students, removed!, index).map((s) => s.first_name)).toEqual(["Léa", "Noé", "Anna", "Tom"]);
  expect(withStudentBack(students.slice(0, 1), removed!, 5).map((s) => s.first_name)).toEqual(["Léa", "Noé"]);
  // Un autre élève ajouté entre-temps (à la fin) : l'élève remis reprend sa place.
  expect(withStudentBack([...students, s("Zoé", null)], removed!, index).map((s) => s.first_name)).toEqual(["Léa", "Noé", "Anna", "Tom", "Zoé"]);
});

test("a quick choice gives its students, and ticked students name their quick choice", () => {
  const students = [s("Léa", "MS", "Les loups"), s("Noé", "GS"), s("Anna", "MS"), s("Tom", "GS", "Les loups ")];
  const names = (list: Student[]) => list.map((s) => s.first_name);
  expect(names(presetStudents(students, "all"))).toEqual(["Léa", "Noé", "Anna", "Tom"]);
  expect(names(presetStudents(students, "level:MS"))).toEqual(["Léa", "Anna"]);
  expect(names(presetStudents(students, "group:Les loups"))).toEqual(["Léa", "Tom"]);
  const presets = ["all", "level:GS", "level:MS", "group:Les loups"];
  expect(presetOf(students, new Set(["Léa", "Noé", "Anna", "Tom"]), presets)).toBe("all");
  expect(presetOf(students, new Set(["Anna", "Léa"]), presets)).toBe("level:MS");
  expect(presetOf(students, new Set(["Léa"]), presets)).toBe("custom");
  expect(presetOf(students, new Set(), presets)).toBe("custom");
});

test("several chosen students get the same level or group at once", () => {
  const students = [s("Léa", "MS", "Les loups"), s("Noé", "GS"), s("Anna", null)];
  const after = patchStudents(students, new Set(["Léa", "Anna"]), { level: "GS" });
  expect(after.map((x) => x.level)).toEqual(["GS", "GS", "GS"]);
  expect(after[0].group).toBe("Les loups");
  expect(patchStudents(students, new Set(["Noé"]), { group: "Les ours" }).map((x) => x.group)).toEqual(["Les loups", "Les ours", ""]);
  expect(patchStudents(students, new Set(), { level: "PS" })).toEqual(students);
});
