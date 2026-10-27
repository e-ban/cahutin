import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

test("every font file in fonts.css exists", () => {
  const css = readFileSync(fileURLToPath(new URL("./fonts.css", import.meta.url)), "utf8");
  const urls = [...css.matchAll(/url\("\/fonts\/([^"]+)"\)/g)].map((m) => m[1]);
  expect(urls.length).toBe(13);
  for (const u of urls) {
    expect(existsSync(fileURLToPath(new URL(`../../assets/fonts/${u}`, import.meta.url))), u).toBe(true);
  }
});
