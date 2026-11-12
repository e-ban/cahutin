// Ce que montre la vue agrandie du tiroir Images pour chacune de ses trois sources : les filtres de la colonne de
// gauche, les groupes de vignettes, et le message quand il n'y a rien. Même forme pour les trois, pour qu'elles se
// présentent de la même façon.
import { assetUrl, classPhotoUrl, myImageUrl } from "./api";
import { fr } from "./i18n/fr";
import { displayName, filterByTheme, themeCounts, type ThemeFilter } from "./myImages";
import { assetsOnPage, byDay, elementAssets, filterPhotos, folderChips, pageBadge, shownFilter, usedAssetIds, type LibraryPhoto, type PhotoFilter } from "./photoLibrary";
import { byDay as libraryByDay, countsByStudent, displayNames, filterLibrary, folderCounts, photoVersion, tagsOf, visiblePhotos, withoutConsent, type LibraryFilter, type Period } from "./phototheque";
import type { ClassPhoto, Document, MyImage, Page, Student } from "./types";

export type Source = "photos" | "library" | "mine";
export interface WideItem {
  key: string;
  url: string;
  title: string;
  badge: string | null;
  warn: string | null;
}
export interface WideGroup {
  title: string;
  items: WideItem[];
}
export interface WideFilter {
  id: string;
  label: string;
  on: boolean;
  heading?: string;
}
export interface WideView {
  filters: WideFilter[];
  groups: WideGroup[];
  empty: string | null;
}

// Photos du document : « all », « unused », « page », « folder:<nom> ».

function photoFilterOf(id: string): PhotoFilter {
  if (id === "unused" || id === "page") return { kind: id };
  if (id.startsWith("folder:")) return { kind: "folder", folder: id.slice("folder:".length) };
  return { kind: "all" };
}

const photoFilterId = (f: PhotoFilter) => (f.kind === "folder" ? `folder:${f.folder}` : f.kind);

export function photosView(photos: LibraryPhoto[], doc: Document, page: Page, filterId: string): WideView {
  const used = usedAssetIds(doc);
  const onPage = assetsOnPage(page);
  const chips = folderChips(photos);
  const filter = shownFilter(photoFilterOf(filterId), chips);
  const current = photoFilterId(filter);
  const pages = new Map<string, number[]>();
  doc.pages.forEach((p, i) => {
    for (const id of new Set(p.elements.flatMap(elementAssets))) pages.set(id, [...(pages.get(id) ?? []), i + 1]);
  });
  const filters: WideFilter[] = [
    { id: "all", label: fr.filterAll(photos.length) },
    { id: "unused", label: fr.filterUnused(photos.filter((p) => !used.has(p.meta.id)).length) },
    { id: "page", label: fr.filterPage(photos.filter((p) => onPage.has(p.meta.id)).length) },
    ...chips.map((c) => ({ id: `folder:${c.folder}`, label: `${c.folder} (${c.count})`, heading: fr.imagesFolders })),
  ].map((f) => ({ ...f, on: f.id === current }));
  const groups = byDay(filterPhotos(photos, filter, used, onPage)).map((d) => ({
    title: d.day ? fr.libraryDayShort(d.day) : fr.photosNoDate,
    items: d.photos.map((p) => ({ key: p.meta.id, url: assetUrl(p.meta.id, true), title: "", badge: pageBadge(pages.get(p.meta.id) ?? []), warn: null })),
  }));
  const empty = groups.length ? null : photos.length === 0 ? fr.imagesNoPhotos : filter.kind === "unused" ? fr.photosAllPlaced : filter.kind === "page" ? fr.photosNoneOnPage : fr.libraryNoneHere;
  return { filters, groups, empty };
}

// Photothèque : « <période>|<filtre> », filtre « all », « untagged », « student:<id> », « tag:<activité> » ou
// « folder:<dossier> ».

const PERIODS: Period[] = ["month", "trimester", "year", "all"];

function libraryFilterOf(id: string): LibraryFilter {
  if (id.startsWith("period:")) return libraryFilterOf(`${id.slice("period:".length)}|all`);
  // Coupé à la première barre seulement : un nom d'activité ou de dossier peut en contenir.
  const bar = id.indexOf("|");
  const [p, f] = bar < 0 ? [id, "all"] : [id.slice(0, bar), id.slice(bar + 1)];
  const period = (PERIODS as string[]).includes(p) ? (p as Period) : "year";
  if (f === "untagged") return { period: "all", untagged: true };
  if (f.startsWith("student:")) return { period, student: f.slice("student:".length) };
  if (f.startsWith("tag:")) return { period, tag: f.slice("tag:".length) };
  if (f.startsWith("folder:")) return { period, folder: f.slice("folder:".length) };
  return { period };
}

export function libraryView(all: ClassPhoto[], students: Student[], filterId: string, today: Date, hasClass: boolean): WideView {
  if (!hasClass) return { filters: [], groups: [], empty: fr.libraryNoClass };
  // Les photos retirées ou absentes restent dans l'index, mais ne sont ni montrées ni comptées.
  const photos = visiblePhotos(all);
  const filter = libraryFilterOf(filterId);
  const p = filter.period;
  const names = displayNames(students);
  const counts = countsByStudent(photos, students, p, today);
  const sorted = [...students].sort((a, b) => (names.get(a.id) ?? "").localeCompare(names.get(b.id) ?? "", "fr", { sensitivity: "base" }));
  const toSort = filterLibrary(photos, { period: "all", untagged: true }, today).length;
  const current = filter.untagged ? `all|untagged` : filter.student ? `${p}|student:${filter.student}` : filter.tag ? `${p}|tag:${filter.tag}` : filter.folder ? `${p}|folder:${filter.folder}` : `${p}|all`;
  // « Toutes » et « À trier » en tête, sans rubrique (comme pour les photos du document), puis les rubriques.
  // Choisir une période garde « Toutes » ; la période est rappelée par son bouton allumé.
  const on = (f: Omit<WideFilter, "on">): WideFilter => ({ ...f, on: f.id === current });
  const filters: WideFilter[] = [
    on({ id: `${p}|all`, label: `${fr.libraryAll} (${filterLibrary(photos, { period: p }, today).length})` }),
    on({ id: "all|untagged", label: `${fr.libraryToSort} (${toSort})` }),
    ...PERIODS.map((x) => ({ id: `period:${x}`, label: fr.libraryPeriods[x], heading: fr.libraryPeriod, on: x === p && !filter.untagged })),
    ...sorted.map((s) => on({ id: `${p}|student:${s.id}`, label: `${names.get(s.id)} (${counts.get(s.id) ?? 0})`, heading: fr.libraryChildren })),
    ...tagsOf(photos).map((t) => on({ id: `${p}|tag:${t}`, label: t, heading: fr.libraryActivities })),
    ...folderCounts(filterLibrary(photos, { period: p }, today)).map((c) =>
      on({ id: `${p}|folder:${c.folder}`, label: `${c.folder} (${c.count})`, heading: fr.imagesFolders }),
    ),
  ];
  const nameOf = (id: string) => names.get(id) ?? "?";
  const groups = libraryByDay(filterLibrary(photos, filter, today)).map((d) => ({
    title: fr.libraryDayShort(d.day),
    items: d.photos.map((ph) => {
      const missing = withoutConsent(ph, students);
      return {
        key: ph.id,
        url: classPhotoUrl(ph.id, true, photoVersion(ph)),
        title: ph.students.map(nameOf).join(", "),
        badge: ph.students.length ? String(ph.students.length) : null,
        warn: missing.length ? fr.libraryNoConsent(missing.map((s) => nameOf(s.id))) : null,
      };
    }),
  }));
  return { filters, groups, empty: groups.length ? null : photos.length ? fr.libraryNoneHere : fr.imagesLibraryEmpty };
}

// Collection : « all », « theme:<thème> », « none » (sans thème).

function themeFilterOf(id: string): ThemeFilter {
  if (id === "none") return { kind: "none" };
  if (id.startsWith("theme:")) return { kind: "theme", theme: id.slice("theme:".length) };
  return { kind: "all" };
}

export function mineView(images: MyImage[], filterId: string): WideView {
  const filter = themeFilterOf(filterId);
  const current = filter.kind === "theme" ? `theme:${filter.theme}` : filter.kind;
  const themes = themeCounts(images);
  const filters: WideFilter[] = [
    { id: "all", label: `${fr.allPictures} (${images.length})` },
    ...themes.map((t) => ({ id: `theme:${t.theme}`, label: `${t.theme} (${t.count})`, heading: fr.imagesThemes })),
    { id: "none", label: fr.imagesNoTheme, heading: fr.imagesThemes },
  ].map((f) => ({ ...f, on: f.id === current }));
  const item = (i: MyImage): WideItem => ({ key: i.name, url: `${myImageUrl(i.name)}?v=${i.modified}`, title: displayName(i.name), badge: null, warn: null });
  const shown = filterByTheme(images, filter);
  // Toute la collection : par premier thème, « Sans thème » à la fin. Un thème choisi : un seul groupe.
  const groups = new Map<string, MyImage[]>();
  for (const i of shown) {
    const title = filter.kind === "theme" ? filter.theme : ([...(i.themes ?? [])].sort((a, b) => a.localeCompare(b, "fr"))[0] ?? fr.imagesNoTheme);
    const list = groups.get(title);
    if (list) list.push(i);
    else groups.set(title, [i]);
  }
  const ordered = [...groups].sort(([a], [b]) => (a === fr.imagesNoTheme ? 1 : b === fr.imagesNoTheme ? -1 : a.localeCompare(b, "fr")));
  return {
    filters,
    groups: ordered.map(([title, list]) => ({ title, items: list.map(item) })),
    empty: shown.length ? null : images.length ? fr.myImagesNone : fr.myImagesEmpty,
  };
}
