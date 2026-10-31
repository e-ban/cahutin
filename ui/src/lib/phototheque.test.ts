import { expect, test } from "vitest";
import { byDay, countsByStudent, displayNames, filterLibrary, periodStart, presence, studentsBehind, tagsOf, withoutConsent } from "./phototheque";
import type { ClassPhoto, Student } from "./types";

const photo = (id: string, taken: string, students: string[] = [], tags: string[] = []): ClassPhoto => ({ id, width_px: 10, height_px: 10, mime: "Jpeg", taken, added: "2026-10-31", students, tags });
const student = (id: string, allowed = true): Student => ({ id, first_name: id, last_name: "", level: null, group: "", photo: null, photo_allowed: allowed, symbol: null });
const today = new Date("2026-10-31T12:00:00Z");

test("periods follow the school year", () => {
  expect(periodStart("month", today)).toBe("2026-10-01");
  expect(periodStart("trimester", today)).toBe("2026-08-01");
  expect(periodStart("year", today)).toBe("2026-09-01");
  expect(periodStart("year", new Date("2027-03-02T00:00:00Z"))).toBe("2026-09-01");
  expect(periodStart("all", today)).toBeNull();
});

test("filters by period, child, activity and photos still to sort", () => {
  const photos = [photo("a", "2026-10-14", ["lea"], ["Ferme"]), photo("b", "2026-09-20", ["noe"]), photo("c", "2026-10-20")];
  expect(filterLibrary(photos, { period: "month" }, today).map((p) => p.id)).toEqual(["c", "a"]);
  expect(filterLibrary(photos, { period: "year", student: "noe" }, today).map((p) => p.id)).toEqual(["b"]);
  expect(filterLibrary(photos, { period: "all", tag: "Ferme" }, today).map((p) => p.id)).toEqual(["a"]);
  expect(filterLibrary(photos, { period: "all", untagged: true }, today).map((p) => p.id)).toEqual(["c"]);
  expect(tagsOf(photos)).toEqual(["Ferme"]);
});

test("children with few photos are pointed out, not before there is enough to compare", () => {
  const kids = ["lea", "noe", "anna", "tom", "zoe"].map((id) => student(id));
  const few = [photo("1", "2026-10-02", ["lea"])];
  expect(studentsBehind(countsByStudent(few, kids, "month", today))).toEqual([]);
  const photos = [
    ...[1, 2, 3, 4].map((i) => photo(`p${i}`, "2026-10-05", ["lea", "noe", "anna"])),
    photo("t", "2026-10-06", ["tom"]),
  ];
  const counts = countsByStudent(photos, kids, "month", today);
  expect(counts.get("lea")).toBe(4);
  expect(studentsBehind(counts)).toEqual(["zoe", "tom"]);
});

test("consent and presence over a selection", () => {
  const p = photo("x", "2026-10-02", ["lea", "noe"]);
  expect(withoutConsent(p, [student("lea"), student("noe", false)]).map((s) => s.id)).toEqual(["noe"]);
  const sel = [p, photo("y", "2026-10-02", ["lea"])];
  expect([presence(sel, "lea"), presence(sel, "noe"), presence(sel, "tom")]).toEqual(["all", "some", "none"]);
});

test("children with the same first name are told apart by their last name", () => {
  const kid = (id: string, first_name: string, last_name: string): Student => ({ ...student(id), first_name, last_name });
  const names = displayNames([kid("1", "Anna", "Durand"), kid("2", "anna", "Martin"), kid("3", "Léo", "Petit"), kid("4", "Noé", "Morel"), kid("5", "Noé", "Moulin"), kid("6", "Zoé", ""), kid("7", "Zoe", "")]);
  expect([...names.values()]).toEqual(["Anna D.", "anna M.", "Léo", "Noé Morel", "Noé Moulin", "Zoé (1)", "Zoe (2)"]);
});

test("photos are grouped by day, in the order given", () => {
  const photos = [photo("a", "2026-10-14"), photo("b", "2026-10-14"), photo("c", "2026-10-02")];
  expect(byDay(photos).map((d) => [d.day, d.photos.map((p) => p.id)])).toEqual([["2026-10-14", ["a", "b"]], ["2026-10-02", ["c"]]]);
});
