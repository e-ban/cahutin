// Mesure le nombre de lignes de chaque cas de fixtures/text_fidelity.json dans Chromium (moteur de
// WebView2) et l'écrit dans expected_lines. Prérequis : `npm run dev` lancé, Chromium de Playwright installé.
// Usage : node scripts/measure-fidelity.mjs [http://localhost:5173/fidelity.html]
import { readFileSync, writeFileSync } from "node:fs";
import { chromium } from "playwright";

const url = process.argv[2] ?? "http://localhost:5173/fidelity.html";
const path = new URL("../fixtures/text_fidelity.json", import.meta.url);
const browser = await chromium.launch();
const page = await browser.newPage();
await page.goto(url);
await page.waitForFunction(() => document.getElementById("result")?.textContent);
const measured = JSON.parse(await page.locator("#result").textContent());
await browser.close();

const cases = JSON.parse(readFileSync(path, "utf8"));
for (const c of cases) c.expected_lines = measured[c.id];
const lines = cases.map((c) => "  " + JSON.stringify(c).replaceAll('":', '": ').replaceAll(',"', ', "'));
writeFileSync(path, "[\n" + lines.join(",\n") + "\n]\n");
console.log(measured);
