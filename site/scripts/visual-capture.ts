// Local visual-QA helper. Requires `puppeteer` (install: `bun add -d puppeteer`)
// and a Chromium-family browser resolved from $BROWSER or PATH (see ./browser).
import puppeteer from "puppeteer";
import { mkdirSync } from "fs";
import { resolveBrowser } from "./browser";

const out = new URL("../.preview", import.meta.url).pathname;
const base = process.env.PREVIEW_BASE ?? "http://localhost:1314/skillprism";

mkdirSync(out, { recursive: true });

const browser = await puppeteer.launch({
  executablePath: resolveBrowser(),
  headless: true,
  args: ["--no-sandbox", "--disable-setuid-sandbox", "--disable-dev-shm-usage"],
});

const shots = [
  { name: "docs-desktop", path: "/docs/quickstart/", w: 1280, h: 160 },
  { name: "docs-mobile", path: "/docs/quickstart/", w: 390, h: 120 },
  { name: "docs-full", path: "/docs/quickstart/", w: 1280, h: 800, full: true },
];

for (const shot of shots) {
  const page = await browser.newPage();
  await page.setViewport({ width: shot.w, height: shot.full ? shot.h : 900 });
  await page.goto(base + shot.path, { waitUntil: "networkidle2", timeout: 15000 });
  const file = out + "/" + shot.name + ".png";
  if (shot.full) await page.screenshot({ path: file });
  else await page.screenshot({ path: file, clip: { x: 0, y: 0, width: shot.w, height: shot.h } });
  console.log("saved", file);
  await page.close();
}

await browser.close();
