import puppeteer from "puppeteer";
import { mkdirSync } from "fs";
import { execSync } from "child_process";

const brave = execSync("command -v brave", { encoding: "utf8" }).trim();
const out = new URL("../.preview", import.meta.url).pathname;
const base = process.env.PREVIEW_BASE ?? "http://localhost:1314/skillprism";

mkdirSync(out, { recursive: true });

const widths = [1280, 1440, 1920] as const;
const pages = [
  { slug: "home", path: "/" },
  { slug: "quickstart", path: "/docs/quickstart/" },
  { slug: "cli", path: "/docs/cli/" },
  { slug: "comparison", path: "/docs/comparison/" },
  { slug: "harnesses", path: "/docs/harnesses/" },
] as const;

type LayoutIssue = { page: string; check: string; detail: string };

const browser = await puppeteer.launch({
  executablePath: brave,
  headless: true,
  args: ["--no-sandbox", "--disable-setuid-sandbox", "--disable-dev-shm-usage"],
});

const saved: string[] = [];
const issues: LayoutIssue[] = [];

for (const pageDef of pages) {
  for (const width of widths) {
    const page = await browser.newPage();
    await page.setViewport({ width, height: 900 });
    await page.goto(base + pageDef.path, { waitUntil: "networkidle2", timeout: 30000 });

    const file = `${out}/stress-${pageDef.slug}-${width}.png`;
    await page.screenshot({ path: file, fullPage: true });
    saved.push(file);
    console.log("saved", file);

    if (width === 1280) {
      const pageIssues = await page.evaluate(() => {
        const out: { check: string; detail: string }[] = [];
        const vw = window.innerWidth;
        const scrollW = document.documentElement.scrollWidth;

        if (scrollW > vw + 1) {
          const offenders: string[] = [];
          for (const el of Array.from(document.querySelectorAll<HTMLElement>("body *"))) {
            const r = el.getBoundingClientRect();
            if (r.right > vw + 1 && r.width > 0 && r.height > 0) {
              const tag = `${el.tagName.toLowerCase()}${el.className ? "." + String(el.className).split(" ").join(".") : ""}`;
              offenders.push(`${tag} right=${Math.round(r.right)}px`);
            }
          }
          out.push({
            check: "horizontal-overflow",
            detail: `scrollWidth ${scrollW}px exceeds viewport ${vw}px by ${scrollW - vw}px${offenders.length ? `; offenders: ${offenders.slice(0, 5).join(", ")}` : ""}`,
          });
        }

        const brand = document.querySelector(".nav-brand") as HTMLElement | null;
        const links = document.querySelector(".nav-links") as HTMLElement | null;
        if (brand && links) {
          const b = brand.getBoundingClientRect();
          const l = links.getBoundingClientRect();
          if (b.left < l.right - 1 && b.right > l.left + 1 && b.top < l.bottom - 1 && b.bottom > l.top + 1) {
            out.push({ check: "nav-overlap", detail: "nav-brand intersects nav-links" });
          }
        }

        const header = document.querySelector(".site-header") as HTMLElement | null;
        if (header) {
          const style = getComputedStyle(header);
          if (style.position !== "sticky") out.push({ check: "sticky-header", detail: `position is ${style.position}` });
          if (Math.abs(parseFloat(style.top)) > 0.5) out.push({ check: "sticky-header-top", detail: `top is ${style.top}` });
        }

        const isDocs = document.body.classList.contains("page-docs");
        if (isDocs && vw > 1100) {
          const sidebar = document.querySelector(".docs-sidebar") as HTMLElement | null;
          const main = document.querySelector(".docs-main") as HTMLElement | null;
          const toc = document.querySelector(".docs-toc") as HTMLElement | null;
          const elems: { name: string; el: HTMLElement }[] = [];
          if (sidebar) elems.push({ name: "sidebar", el: sidebar });
          if (main) elems.push({ name: "main", el: main });
          if (toc && getComputedStyle(toc).display !== "none") elems.push({ name: "toc", el: toc });

          for (let i = 0; i < elems.length; i++) {
            for (let j = i + 1; j < elems.length; j++) {
              const a = elems[i].el.getBoundingClientRect();
              const b = elems[j].el.getBoundingClientRect();
              if (a.left < b.right - 1 && a.right > b.left + 1 && a.top < b.bottom - 1 && a.bottom > b.top + 1) {
                out.push({ check: "docs-shell-overlap", detail: `${elems[i].name} overlaps ${elems[j].name}` });
              }
            }
          }

          const navH = header?.offsetHeight ?? 56;
          for (const { name, el } of elems) {
            if (name === "main") continue;
            const style = getComputedStyle(el);
            if (style.position !== "sticky") out.push({ check: "sticky-docs-panel", detail: `${name} position is ${style.position}` });
            if (Math.abs(parseFloat(style.top) - navH) > 1) {
              out.push({ check: "sticky-docs-top", detail: `${name} top is ${style.top}, expected header height ${navH}px` });
            }
          }
        }

        if (isDocs) {
          const main = document.querySelector(".docs-main") as HTMLElement | null;
          if (main) {
            const mainRect = main.getBoundingClientRect();
            main.querySelectorAll("table").forEach((table, idx) => {
              const t = table.getBoundingClientRect();
              if (t.right > mainRect.right + 1) {
                out.push({
                  check: "table-overflow",
                  detail: `table #${idx + 1} right ${Math.round(t.right)}px exceeds main right ${Math.round(mainRect.right)}px by ${Math.round(t.right - mainRect.right)}px`,
                });
              }
            });
          }
        }

        return out;
      });

      for (const i of pageIssues) issues.push({ page: pageDef.slug, ...i });
    }

    await page.close();
  }
}

await browser.close();

console.log("\n=== Desktop stress test report ===\n");
console.log(`Screenshots saved: ${saved.length}`);
for (const f of saved) console.log(`  ${f}`);

if (issues.length === 0) {
  console.log("\nNo layout issues found at 1280px.");
} else {
  console.log(`\nLayout issues found: ${issues.length}\n`);
  for (const issue of issues) console.log(`[${issue.page}] ${issue.check}: ${issue.detail}`);
  process.exit(1);
}
