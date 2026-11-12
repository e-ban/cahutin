import { expect, test } from "vitest";
import { baseName, byDay, foldersWithOldCopies, upsertPhotos, countsByStudent, displayNames, filterLibrary, folderCounts, isOldCopy, oldCopies, periodStart, presence, studentsBehind, tagsOf, missingConsent, visiblePhotos, withoutConsent } from "./phototheque";
import type { ClassPhoto, Student } from "./types";

const photo = (id: string, taken: string, students: string[] = [], tags: string[] = [], more: Partial<ClassPhoto> = {}): ClassPhoto => ({
  id,
  width_px: 10,
  height_px: 10,
  mime: "Jpeg",
  taken,
  added: "2026-10-31",
  students,
  tags,
  path: `/photos/IMG_${id}.jpg`,
  folder: "Photos",
  size: 100,
  modified: 1,
  fingerprint: "0",
  hidden: false,
  missing: false,
  ...more,
});
const student = (id: string, allowed = true): Student => ({ id, first_name: id, last_name: "", level: null, group: "", photo: null, photo_allowed: allowed, symbol: null });
const today = new Date("2026-10-31T12:00:00Z");

test("missingConsent réunit les enfants sans autorisation de plusieurs photos, une fois chacun", () => {
  const students = [student("Léo"), student("Inès", false), student("Tom", false)];
  const photos = [photo("x", "2026-10-05", ["Inès"]), photo("y", "2026-10-05", ["Léo"]), photo("z", "2026-10-06", ["Tom", "Inès"])];
  expect(missingConsent(photos, students, ["x", "y", "z"]).map((s) => s.id)).toEqual(["Inès", "Tom"]);
  expect(missingConsent(photos, students, ["y"])).toEqual([]);
});

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

test("seules les photos ni retirées ni absentes sont montrées", () => {
  const photos = [photo("a", "2026-10-05"), photo("b", "2026-10-05", [], [], { hidden: true }), photo("c", "2026-10-05", [], [], { missing: true })];
  expect(visiblePhotos(photos).map((p) => p.id)).toEqual(["a"]);
});

test("filtre par dossier, et nombre de photos de chaque dossier, par ordre alphabétique", () => {
  const photos = [
    photo("a", "2026-10-14", [], [], { folder: "Sortie | ferme" }),
    photo("b", "2026-10-12", [], [], { folder: "Atelier" }),
    photo("c", "2026-10-10", [], [], { folder: "Sortie | ferme" }),
  ];
  expect(filterLibrary(photos, { period: "all", folder: "Sortie | ferme" }, today).map((p) => p.id)).toEqual(["a", "c"]);
  expect(filterLibrary(photos, { period: "all", folder: null }, today).map((p) => p.id)).toEqual(["a", "b", "c"]);
  expect(folderCounts(photos)).toEqual([
    { folder: "Atelier", count: 1 },
    { folder: "Sortie | ferme", count: 2 },
  ]);
});

test("le nom d'un dossier ou d'un fichier, sous Windows comme ailleurs", () => {
  expect(baseName("/home/anne/Images/Sortie")).toBe("Sortie");
  expect(baseName("C:\\Users\\anne\\Images\\Sortie\\")).toBe("Sortie");
  expect(baseName("D:\\photo.jpg")).toBe("photo.jpg");
});

test("une ancienne copie porte pour nom l'identifiant de la photo", () => {
  const copy = photo("3f2a", "2026-10-05", [], [], { path: "/Documents/Cahutin/Photos/MS/3f2a.jpg" });
  const winCopy = photo("9b", "2026-10-05", [], [], { path: "C:\\Cahutin\\Photos\\MS\\9b.webp" });
  const original = photo("3f2b", "2026-10-05", [], [], { path: "/Images/3f2b-ferme.jpg" });
  const hiddenCopy = photo("77", "2026-10-05", [], [], { path: "/Photos/MS/77.jpg", hidden: true });
  expect([copy, winCopy, original].map(isOldCopy)).toEqual([true, true, false]);
  expect(oldCopies([copy, winCopy, original, hiddenCopy]).map((p) => p.id)).toEqual(["3f2a", "9b"]);
});

test("le dossier des anciennes copies ne peut pas être oublié tant qu'il en reste", () => {
  const folders = ["/Documents/Cahutin/Photos/MS", "/Documents/Cahutin/Photos/MS-ferme", "C:\\Images"];
  const copy = photo("3f2a", "2026-10-05", [], [], { path: "/Documents/Cahutin/Photos/MS/3f2a.jpg" });
  const original = photo("b1", "2026-10-05", [], [], { path: "/Documents/Cahutin/Photos/MS-ferme/IMG_1.jpg" });
  const winCopy = photo("9b", "2026-10-05", [], [], { path: "C:\\Images\\9b.jpg" });
  expect(foldersWithOldCopies([copy, original, winCopy], folders)).toEqual(["/Documents/Cahutin/Photos/MS", "C:\\Images"]);
  // Migration faite : les copies sont retirées, le dossier peut de nouveau être oublié.
  expect(foldersWithOldCopies([{ ...copy, hidden: true }, original], folders)).toEqual([]);
});

test("un lot reçu pendant le parcours remplace les photos connues et ajoute les nouvelles", () => {
  const photos = [photo("a", "2026-10-05"), photo("b", "2026-10-06")];
  const moved = photo("a", "2026-10-05", [], [], { path: "/ailleurs/IMG_a.jpg" });
  const out = upsertPhotos(photos, [photo("c", "2026-10-07"), moved]);
  expect(out.map((p) => p.id)).toEqual(["a", "b", "c"]);
  expect(out[0].path).toBe("/ailleurs/IMG_a.jpg");
  expect(photos[0].path).toBe("/photos/IMG_a.jpg");
});
