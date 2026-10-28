import { expect, test } from "vitest";
import { addRecent, type Recent } from "./recents";

const r = (path: string, openedAt: number): Recent => ({ path, title: path, openedAt });

test("addRecent puts the newest first and replaces the same path", () => {
  const list = addRecent(addRecent([], r("/a.carnet", 1)), r("/b.carnet", 2));
  expect(list.map((x) => x.path)).toEqual(["/b.carnet", "/a.carnet"]);
  const again = addRecent(list, { path: "/a.carnet", title: "A renommé", openedAt: 3 });
  expect(again.map((x) => x.path)).toEqual(["/a.carnet", "/b.carnet"]);
  expect(again[0].title).toBe("A renommé");
});

test("addRecent keeps at most 8 documents", () => {
  let list: Recent[] = [];
  for (let i = 0; i < 10; i++) list = addRecent(list, r(`/${i}.carnet`, i));
  expect(list).toHaveLength(8);
  expect(list[0].path).toBe("/9.carnet");
  expect(list[7].path).toBe("/2.carnet");
});
