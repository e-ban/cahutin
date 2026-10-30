import { expect, test } from "vitest";
import { filterStudents, groups, schoolYear } from "./classStore.svelte";
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
