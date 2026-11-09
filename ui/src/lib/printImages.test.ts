import { expect, test } from "vitest";
import { imagesDecoded } from "./printImages";

/** L'impression attend que chaque photo entière soit chargée ; une image en erreur n'empêche pas d'imprimer. */
test("printing waits until every image is decoded, broken ones included", async () => {
  let release: () => void = () => {};
  const slow = { decode: () => new Promise<void>((r) => (release = r)) };
  const broken = { decode: () => Promise.reject(new Error("abîmée")) };
  let done = false;
  const waiting = imagesDecoded([slow, broken]).then(() => (done = true));
  await Promise.resolve();
  expect(done).toBe(false);
  release();
  await waiting;
  expect(done).toBe(true);
});
